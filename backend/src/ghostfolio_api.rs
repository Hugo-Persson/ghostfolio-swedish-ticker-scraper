use crate::AvanzaGetFundInfo::AvanzaFundInfo;

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
                "weight": (c.y as i32)/100
            })
        })
        .collect();
    serde_json::to_string(&countries).expect("Could not create json country json")
}

fn get_current_date() -> String {
    todo!("Get current date")
}

fn generate_url(info: &AvanzaFundInfo) -> String {
    format!("https://www.avanza.se/fonder/om-fonden.html/{}", "dd")
}

fn scraper_config(info: &AvanzaFundInfo) -> Result<std::string::String, serde_json::Error> {
    let value = json!({
        "source": "avanza",
        "orderbook_id": "1"
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
                "weight": (s.y as i32)/100,
            })
        })
        .collect();
    serde_json::to_string(&sectors)
}
fn generate_ticker_name(info: &AvanzaFundInfo) -> String {
    info.name.replace(" ", "_").to_uppercase()
}

pub fn prepare_insert_fund(info: AvanzaFundInfo) -> Result<std::string::String, serde_json::Error> {
    // TODO: bind instead of insert directly
    let query = format!(
        r#"INSERT INTO public."SymbolProfile" (countries, "createdAt", "dataSource", id, name, "updatedAt", symbol, sectors,
                                    currency, "assetClass", "assetSubClass", "symbolMapping", "scraperConfiguration",
                                    url, comment, isin)
VALUES ('{}', '{}', 'MANUAL'::"DataSource", null, '{}',
        '{}', '{}', '{}', 'SEK',
        'EQUITY'::"AssetClass", 'MUTUALFUND'::"AssetSubClass", null, {},
        '{}', null, '{}');
"#,
        generate_country_json(&info),
        get_current_date(),
        info.name,
        get_current_date(),
        generate_ticker_name(&info),
        generate_sectors_json(&info)?,
        scraper_config(&info)?,
        generate_url(&info),
        info.isin,
    );
    Ok(query)
}
