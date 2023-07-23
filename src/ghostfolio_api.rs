use core::fmt;

use crate::avanza_get_fund_info::AvanzaFundInfo;
use crate::avanza_get_stock_info::AvanzaStockInfo;
use crate::GhostfolioDB;
use serde::Serialize;
use uuid::Uuid;

use rocket_db_pools::sqlx::Row;
use rocket_db_pools::Connection;

use rocket_db_pools::sqlx;
use serde_json::json;
fn generate_country_json(info: &AvanzaFundInfo) -> String {
    if info.country_chart_data.len() == 0 {
        return "[]".to_string();
    }

    let countries: Vec<serde_json::Value> = info
        .country_chart_data
        .iter()
        .map(|c| {
            json!({
                "code": c.country_code,
                "weight": c.y/(100 as f64)
            })
        })
        .collect();
    serde_json::to_string(&countries).expect("Could not create json country json")
}

fn get_current_date() -> String {
    todo!("Get current date")
}

fn generate_url(orderbook_id: &String) -> String {
    format!(
        "https://www.avanza.se/fonder/om-fonden.html/{}",
        orderbook_id
    )
}

#[derive(Serialize, PartialEq)]
pub enum SymbolType {
    STOCK,
    #[serde(rename = "FUND")]
    MUTUALFUND,
}
impl fmt::Display for SymbolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::STOCK => write!(f, "STOCK"),
            Self::MUTUALFUND => write!(f, "FUND"),
        }
    }
}

fn scraper_config(
    orderbook_id: &String,
    symbol_type: SymbolType,
) -> Result<std::string::String, serde_json::Error> {
    let value = json!({
        "source": "avanza",
        "orderbook_id": orderbook_id,
        "symbol_type": symbol_type
    });
    serde_json::to_string(&value)
}
fn generate_sectors_json(info: &AvanzaFundInfo) -> Result<std::string::String, serde_json::Error> {
    if info.sector_chart_data.len() == 0 {
        return Ok("[]".to_string());
    }
    let sectors: Vec<serde_json::Value> = info
        .sector_chart_data
        .iter()
        .map(|s| {
            json!({
                "name": s.name,
                "weight": s.y /(100 as f64),
            })
        })
        .collect();
    serde_json::to_string(&sectors)
}
fn generate_ticker_name(name: &String) -> String {
    name.replace(" ", "_").to_uppercase()
}
fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Returns the query to insert a symbol into the database
/// # Arguments
/// 1. `companies` - The companies that the symbol invest in, as a json string
/// 2. `id` - The id of the symbol
/// 3. `name` - The name of the symbol
/// 4. `symbol` - The symbol of the symbol
/// 5. `sectors` - The sectors that the symbol invest in, as a json string
/// 6. `scraper_configuration` - The scraper configuration of the symbol
/// 7. `url` - The url to the symbol
/// 8. `isin` - The isin of the symbol
fn get_insert_symbol_query() -> &'static str {
    r#"INSERT INTO public."SymbolProfile" (countries, "createdAt", "dataSource", id, name, "updatedAt", symbol, sectors,
    currency, "assetClass", "assetSubClass", "symbolMapping", "scraperConfiguration",
    url, comment, isin) VALUES 
    ($1, current_timestamp, 'MANUAL'::"DataSource", $2, $3, current_timestamp, $4, $5, 'SEK', 'EQUITY'::"AssetClass", 'MUTUALFUND'::"AssetSubClass", null, $6,
$7, null, $8);
"#
}

pub fn prepare_insert_fund(
    info: AvanzaFundInfo,
    orderbook_id: &String,
) -> Result<std::string::String, serde_json::Error> {
    // TODO: bind instead of insert directly
    let query = format!(
        r#"INSERT INTO public."SymbolProfile" (countries, "createdAt", "dataSource", id, name, "updatedAt", symbol, sectors,
                                    currency, "assetClass", "assetSubClass", "symbolMapping", "scraperConfiguration",
                                    url, comment, isin)
VALUES ('{}', current_timestamp, 'MANUAL'::"DataSource", '{}', '{}',
        current_timestamp, '{}', '{}', 'SEK',
        'EQUITY'::"AssetClass", 'MUTUALFUND'::"AssetSubClass", null, '{}',
        '{}', null, '{}');
"#,
        generate_country_json(&info),
        generate_id(),
        info.name,
        generate_ticker_name(&info.name),
        generate_sectors_json(&info)?,
        scraper_config(orderbook_id, SymbolType::MUTUALFUND)?,
        generate_url(orderbook_id),
        info.isin,
    );
    Ok(query)
}

pub struct GhostfolioAPI {
    db: Connection<GhostfolioDB>,
}

#[derive(Debug)]
pub struct SymbolInfo {
    pub id: String,
    pub orderbook_id: String,
    pub symbol: String,
}

impl SymbolType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "STOCK" => Self::STOCK,
            "FUND" => Self::MUTUALFUND,
            _ => panic!("Unknown symbol type"),
        }
    }
}
#[derive(Debug)]
pub struct MarketData {
    pub symbol: String,
    pub market_price: f64,
}
impl GhostfolioAPI {
    pub fn new(db: Connection<GhostfolioDB>) -> Self {
        Self { db: db }
    }

    pub async fn isin_exists(&mut self, isin: &String) -> bool {
        let query = r#"SELECT id FROM "SymbolProfile" WHERE isin = '?' LIMIT 1;"#;
        let res = sqlx::query(query).bind(isin).fetch_one(&mut *self.db).await;
        res.is_ok()
    }

    pub async fn get_isin_in_db(&mut self) -> Vec<String> {
        let query = r#"SELECT isin FROM "SymbolProfile";"#;
        let res = sqlx::query(query).fetch_all(&mut *self.db).await;
        match res {
            Ok(rows) => rows
                .iter()
                .map(|row| row.try_get("isin"))
                .filter_map(|x| x.ok())
                .collect(),
            Err(err) => {
                println!("Error: {}", err);
                vec![]
            }
        }
    }

    pub async fn insert_fund(
        &mut self,
        info: AvanzaFundInfo,
        orderbook_id: &String,
    ) -> Result<(), rocket_db_pools::sqlx::Error> {
        let query = prepare_insert_fund(info, orderbook_id).expect("Could not get query");
        println!("Query {}", query);
        let res = sqlx::query(query.as_str()).execute(&mut *self.db).await;
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get_our_tickers(&mut self) -> Vec<SymbolInfo> {
        let query = r#"SELECT id, symbol, "scraperConfiguration" ->> 'orderbook_id' AS orderbook_id FROM "SymbolProfile" WHERE
                                                                                            "scraperConfiguration" IS NOT NULL
                                                                                        AND "scraperConfiguration" ->> 'source' = 'avanza';"#;
        let res = sqlx::query(query).fetch_all(&mut *self.db).await;
        match res {
            Ok(rows) => {
                let symbols: Vec<SymbolInfo> = rows
                    .iter()
                    .map(|row| {
                        let id: String = row.try_get("id").expect("Could not get id");
                        let orderbook_id: String = row
                            .try_get("orderbook_id")
                            .expect("Could not get orderbook_id");
                        let symbol = row.try_get("symbol").expect("Could not get symbol");
                        SymbolInfo {
                            id,
                            orderbook_id,
                            symbol,
                        }
                    })
                    .collect();
                symbols
            }
            Err(err) => {
                println!("Error: {}", err);
                vec![]
            }
        }
    }

    pub async fn insert_market_data(
        &mut self,
        data: MarketData,
    ) -> Result<(), rocket_db_pools::sqlx::Error> {
        let query = r#"INSERT INTO public."MarketData" ("createdAt", date, id, symbol, "marketPrice", "dataSource", state) VALUES ('2023-07-20 20:01:14.000', '2023-07-20 20:01:16.000', $1, $2, $3, 'MANUAL'::"DataSource", 'CLOSE'::"MarketDataState");"#;
        let res = sqlx::query(query)
            .bind(generate_id())
            .bind(data.symbol)
            .bind(data.market_price)
            .execute(&mut *self.db)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn insert_stock_symbol(
        &mut self,
        data: AvanzaStockInfo,
    ) -> Result<(), rocket_db_pools::sqlx::Error> {
        let query = get_insert_symbol_query();
        let res = sqlx::query(query)
            .bind("[]")
            .bind(generate_id())
            .bind(&data.name)
            .bind(generate_ticker_name(&data.name))
            .bind("[]")
            .bind(scraper_config(&data.orderbook_id, SymbolType::STOCK).unwrap())
            .bind(generate_url(&data.orderbook_id))
            .bind(data.isin)
            .execute(&mut *self.db)
            .await?;
        Ok(())
    }
}
