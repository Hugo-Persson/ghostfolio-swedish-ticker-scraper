#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

mod AvanzaGetFundInfo;
mod AvanzaSearch;
mod ghostfolio_api;

use crate::ghostfolio_api::prepare_insert_fund;
use crate::AvanzaGetFundInfo::get_avanza_fund_info;
use crate::AvanzaSearch::{search_avanza, Hit};
use csv::{Reader, ReaderBuilder, StringRecord};
use dotenv::dotenv;
use futures::stream::StreamExt;
use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::local::asynchronous::Client;
use rocket::serde::{json::Json, Deserialize};
use rocket::{tokio, Build, Data, Orbit, Request, Response, Rocket};
use rocket_db_pools::{sqlx, Database};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use serde_json::json;
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
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 15));
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
    let mut response = client
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
async fn init_form(
    mut db: Connection<GhostfolioDB>,
    mut data: Form<InitTickersFormData<'_>>,
) -> Template {
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
    let mut hits_result: Vec<Hit> = vec![];
    for hit in hits_async {
        let res = hit.await;
        match res {
            Ok((_isin, hit)) => {
                hits_result.push(hit);
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
#[post("/select-tickers", data = "<data>")]
async fn select_tickers(
    mut db: Connection<GhostfolioDB>,
    data: Form<SelectTickersForm>,
) -> &'static str {
    println!("Data: {:#?}", data);
    let mut ghost_api = ghostfolio_api::GhostfolioAPI::new(db);
    for id in &data.ids {
        let info = get_avanza_fund_info(id).await.unwrap();
        let res = ghost_api.insert_fund(info, id).await;

        match res {
            Ok(_) => println!("Ok"),
            Err(err) => println!("Error: {}", err),
        }
    }

    return "Success, done";
}

#[get("/perform-scrape")]
async fn perform_scrape() -> &'static str {
    "Hello"
}
