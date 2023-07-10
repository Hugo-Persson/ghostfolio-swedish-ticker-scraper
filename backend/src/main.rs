#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

mod AvanzaSearch;

use std::path::Path;

use csv::{Reader, ReaderBuilder, StringRecord};
use futures::future::join_all;
use futures::stream::StreamExt;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::serde::{json::Json, Deserialize};
use rocket::Request;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use serde_json::json;

use crate::AvanzaSearch::{search_avanza, Hit};

// extern crate rocket_multipart_form_data;
//
// use rocket::http::ContentType;
// use rocket::Data;
//
// use rocket_multipart_form_data::{
//     MultipartFormData, MultipartFormDataField, MultipartFormDataOptions,
// };

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/api", routes![hello])
        .mount("/", routes![index, init, init_form])
        .attach(Template::fairing())
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
    tickers.dedup();
    return tickers;
}

#[post("/init", data = "<data>")]
async fn init_form(mut data: Form<InitTickersFormData<'_>>) -> Template {
    println!("Column: {}", data.isin_column);
    // println!("File: {:#?} ", data.csv);

    let tickers = get_tickers_from_csv(data.csv.path().unwrap(), &data.isin_column);
    let hits_async = tickers[1..3].iter().map(search_avanza); // We only take 10 at a time. Buld
    let mut hits_result: Vec<Hit> = vec![];
    for hit in hits_async {
        let res = hit.await;
        if res.is_ok() {
            hits_result.push(res.unwrap());
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

#[derive(FromForm)]
struct SelectTickersForm<'r> {
    ids: Vec<String>,
}
#[post("/init", data = "<data>")]
fn select_tickers(data: Form<SelectTickersForm<'_>>) -> Template {
    todo!("Finish")
}
