#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

mod avanza_get_fund_info;
mod avanza_get_stock_info;
mod avanza_search;
mod ghostfolio_api;

use crate::avanza_get_fund_info::get_avanza_fund_info;
use crate::avanza_search::{search_avanza, Hit};
use crate::ghostfolio_api::{MarketData, SymbolType};
use csv::{ReaderBuilder, StringRecord};
use dotenv::dotenv;
use futures::stream::StreamExt;
use ghostfolio_api::SymbolInfo;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::form::Form;
use rocket::fs::TempFile;

use rocket::serde::{json::Json, Deserialize};
use rocket::{tokio, Orbit, Request, Rocket};
use rocket_db_pools::{sqlx, Database};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::path::Path;

#[derive(Database)]
#[database("postgres_ghostfolio")]
pub struct GhostfolioDB(sqlx::PgPool);

use rocket_db_pools::sqlx::Row;
use rocket_db_pools::Connection;

// extern crate rocket_multipart_form_data;
//
// use rocket::http::ContentType;
// use rocket::Data;
//
// use rocket_multipart_form_data::{
//     MultipartFormData, MultipartFormDataField, MultipartFormDataOptions,
// };

struct PostLaunchFairing {}

#[rocket::async_trait]
impl Fairing for PostLaunchFairing {
    fn info(&self) -> Info {
        Info {
            name: "Post Launch Fairing",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, _rocket: &Rocket<Orbit>) {
        println!("Liftoff!");
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60)); // Once an hour
            let client: reqwest::Client = reqwest::Client::new();
            loop {
                interval.tick().await;
                println!("Scraping");
                scrape_job(&client).await;
            }
        });

        //let mut response = client.get(uri!(hello)).dispatch();
        //println!("Response: {}", response.await.into_string().await.unwrap());
    }
}

async fn scrape_job(client: &reqwest::Client) -> () {
    // Call localhost:8000/perform-scrape
    let response = client
        .get("http://localhost:8000/perform-scrape")
        .send()
        .await;
    match response {
        Ok(response) => {
            println!("Response: {:?}", response.text().await);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    rocket::build()
        .attach(GhostfolioDB::init())
        .mount("/api", routes![hello])
        .mount(
            "/",
            routes![index, init, init_form, select_tickers, perform_scrape],
        )
        .attach(Template::fairing())
        .attach(PostLaunchFairing {})
}

#[get("/hello")]
fn hello() -> Json<&'static str> {
    Json(
        "{
    'status': 'success',
    'message': 'Hello API!'
  }",
    )
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Oh no! We couldn't find the requested path '{}'", req.uri())
}

#[derive(Serialize)]
struct Context {
    first_name: String,
    last_name: String,
}

#[get("/")]
fn index() -> Template {
    Template::render("index", context!())
}

#[get("/init")]
fn init() -> Template {
    let context = json!({});
    Template::render("init", context)
}

fn missing_data_init() -> Template {
    todo!("Finish")
}

#[derive(Deserialize)]
struct InitTickersData {
    csv_content: String,
    isin_column: String,
}

#[post("/init", data = "<data>")]
fn init_submit(data: Json<InitTickersData>) -> String {
    let d: InitTickersData = data.into_inner();
    return format!("Done, column {}", d.isin_column).to_string();
}

#[derive(FromForm)]
struct InitTickersFormData<'r> {
    isin_column: String,
    csv: TempFile<'r>,
}

// fn extract_column_from_csn(column_index: usize, row: StringRecord) -> &'static str {
//     let col_val = row.get(column_index).unwrap();
//     return col_val;
// }
fn extract_column_from_csn(column_index: usize, row: StringRecord) -> String {
    let col_val = row.get(column_index).unwrap().to_string();
    return col_val;
}

fn get_tickers_from_csv<'a>(path: &'a Path, isin_column: &'a String) -> Vec<String> {
    let mut csv_reader = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .unwrap();
    println!("Headers: {:#?} ", csv_reader.headers());
    println!("ISIN column: {}", isin_column);
    let index_of_isin = csv_reader
        .headers()
        .unwrap()
        .iter()
        .position(|r| r == isin_column)
        .unwrap();
    let mut tickers: Vec<String> = csv_reader
        .records()
        .filter(|e| e.is_ok())
        .map(|e| extract_column_from_csn(index_of_isin, e.unwrap()))
        .filter(|e| e != &"-".to_string())
        .collect();
    tickers.sort_unstable();
    tickers.dedup();
    return tickers;
}

#[post("/init", data = "<data>")]
async fn init_form(db: Connection<GhostfolioDB>, data: Form<InitTickersFormData<'_>>) -> Template {
    println!("Column: {}", data.isin_column);
    // println!("File: {:#?} ", data.csv);

    let tickers = get_tickers_from_csv(data.csv.path().unwrap(), &data.isin_column);
    let mut ghost_api = ghostfolio_api::GhostfolioAPI::new(db);
    let isin_in_db = ghost_api.get_isin_in_db().await;
    println!("ISIN in DB: {:#?}", isin_in_db);
    println!("Tickers: {:#?}", tickers);
    let hits_async = tickers
        .iter()
        .filter(|x| !isin_in_db.contains(x))
        .take(10)
        .map(search_avanza); // We only take 10 at a time. Buld
    let mut hits_result: Vec<(String, Hit)> = vec![];
    for hit in hits_async {
        let res = hit.await;
        match res {
            Ok((symbol, hit)) => {
                hits_result.push((symbol.to_string(), hit));
            }
            Err(err) => println!("Error: {}", err),
        }
    }

    println!("{:#?}", tickers);
    info!("Result from avanza is: {:#?}", hits_result);

    Template::render(
        "select-tickers",
        context! {
            hits: hits_result
        },
    )
}

#[derive(FromForm, Debug)]
struct SelectTickersForm {
    ids: Vec<String>,
}

async fn select_stock(
    orderbook_id: &String,
    ghost_api: &mut ghostfolio_api::GhostfolioAPI,
) -> Result<(), Box<dyn Error>> {
    let stock_info = avanza_get_stock_info::avanza_get_stock_info(&orderbook_id).await?;
    let _ = ghost_api.insert_stock_symbol(stock_info).await?;
    Ok(())
}

async fn select_fund(
    orderbook_id: &String,
    ghost_api: &mut ghostfolio_api::GhostfolioAPI,
) -> Result<(), Box<dyn Error>> {
    let fund_info = get_avanza_fund_info(&orderbook_id).await?;
    let _ = ghost_api.insert_fund(fund_info, &orderbook_id).await;
    Ok(())
}

#[post("/select-tickers", data = "<data>")]
async fn select_tickers(
    db: Connection<GhostfolioDB>,
    data: Form<SelectTickersForm>,
) -> &'static str {
    println!("Data: {:#?}", data);
    let mut ghost_api = ghostfolio_api::GhostfolioAPI::new(db);
    for id_symbol in &data.ids {
        let mut split_result = id_symbol.split("-");
        let id = split_result.next().unwrap();
        let symbol = SymbolType::from_str(split_result.next().unwrap());

        if symbol == SymbolType::STOCK {
            let res = select_stock(&id.to_string(), &mut ghost_api).await;
            match res {
                Ok(_) => println!("Ok, stock inserted. id: {}", id),
                Err(err) => println!("Error: {}", err),
            }
        } else if symbol == SymbolType::MUTUALFUND {
            let res = select_fund(&id.to_string(), &mut ghost_api).await;
            match res {
                Ok(_) => println!("Ok, fund inserted. id: {}", id),
                Err(err) => println!("Error, fund not inserted, id: {}, message: {}", id, err),
            }
        }
    }

    return "Success, done";
}

async fn get_market_data_for_stock(target: &SymbolInfo) -> MarketData {
    let stock_info = avanza_get_stock_info::avanza_get_stock_info(&target.orderbook_id)
        .await
        .unwrap();
    println!("Stock info: {:#?}", stock_info);
    MarketData {
        symbol: target.symbol.clone(),
        market_price: stock_info.quote.last,
    }
}
async fn get_market_data_for_fund(target: &SymbolInfo) -> MarketData {
    let fund_info = get_avanza_fund_info(&target.orderbook_id).await.unwrap();
    MarketData {
        symbol: target.symbol.clone(),
        market_price: fund_info.nav,
    }
}

#[get("/perform-scrape")]
async fn perform_scrape(db: Connection<GhostfolioDB>) -> &'static str {
    let mut ghost_api = ghostfolio_api::GhostfolioAPI::new(db);
    let scrape_targets = ghost_api.get_our_tickers().await;
    for target in &scrape_targets {
        let data = match target.symbol_type {
            SymbolType::STOCK => get_market_data_for_stock(target).await,
            SymbolType::MUTUALFUND => get_market_data_for_fund(target).await,
        };
        match ghost_api.insert_market_data(data).await {
            Ok(_) => println!("Ok"),
            Err(err) => println!("Error: {}", err),
        }
    }
    "Hello"
}
