use crate::AvanzaGetFundInfo::AvanzaFundInfo;
use crate::GhostfolioDB;
use uuid::Uuid;

use rocket_db_pools::sqlx::Row;
use rocket_db_pools::Connection;

use rocket_db_pools::{sqlx, Database};
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

fn scraper_config(orderbook_id: &String) -> Result<std::string::String, serde_json::Error> {
    let value = json!({
        "source": "avanza",
        "orderbook_id": orderbook_id
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
fn generate_ticker_name(info: &AvanzaFundInfo) -> String {
    info.name.replace(" ", "_").to_uppercase()
}
fn generate_id() -> String {
    Uuid::new_v4().to_string()
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
        generate_ticker_name(&info),
        generate_sectors_json(&info)?,
        scraper_config(orderbook_id)?,
        generate_url(orderbook_id),
        info.isin,
    );
    Ok(query)
}

pub async fn isin_exists(isin: &String, mut db: &mut Connection<GhostfolioDB>) -> bool {
    let query = r#"SELECT id FROM "SymbolProfile" WHERE isin = '?' LIMIT 1;"#;
    let res = sqlx::query(query).bind(isin).fetch_one(db).await;
    res.is_ok()
}
