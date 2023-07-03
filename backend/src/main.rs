#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use rocket::serde::{json::Json, Deserialize};
use rocket::{response::content::Json, Request};
use rocket_contrib::{json, templates::Template};
use serde::Serialize;

// extern crate rocket_multipart_form_data;
//
// use rocket::http::ContentType;
// use rocket::Data;
//
// use rocket_multipart_form_data::{
//     MultipartFormData, MultipartFormDataField, MultipartFormDataOptions,
// };

fn main() {
    rocket::ignite()
        .register(catchers![not_found])
        .mount("/api", routes![hello])
        .mount("/", routes![index, init, init_submit])
        .attach(Template::fairing())
        .launch();
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
    #[derive(Serialize)]
    struct Context {
        first_name: String,
        last_name: String,
    }
    let context = Context {
        first_name: String::from("Ebenezer"),
        last_name: String::from("Don"),
    };
    Template::render("index", context)
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
#[serde(crate = "rocket::serde")]
struct Tic<'r> {
    description: &'r str,
    complete: bool,
}

#[post("/init", data = "<tickers>")]
fn init_submit(tickers: Json<Vec<String>>) -> String {
    let parsed_tickers = serde_json::from_value(tickers);
    for ticker in parsed_tickers {
        println!("{}", ticker);
    }
    return "Done".to_string();
}

// #[post("/init", data = "<data>")]
// fn init_submit(content_type: &ContentType, data: Data) -> Template {
//     let mut options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
//         MultipartFormDataField::file("csv")
//             .content_type_by_string(Some("text/csv"))
//             .unwrap(),
//         MultipartFormDataField::text("isinColumn"),
//     ]);
//     let mut multipart_form_data = MultipartFormData::parse(content_type, data, options).unwrap();
//
//     let csv_arr = match multipart_form_data.files.get("csv") {
//         Some(file) => file,
//         None => return missing_data_init(),
//     };
//     let csv = &csv_arr[0];
//     let isin_column_arr = match multipart_form_data.texts.remove("isinColumn") {
//         Some(data) => data,
//         None => return missing_data_init(),
//     };
//
//     let isin_column = &isin_column_arr[0];
//
//     #[derive(Serialize)]
//     struct Context {
//         column: String,
//         csv_content: String,
//     }
//
//     let csv_content = csv.file_name.clone().unwrap_or("err".to_string());
//     let context = Context {
//         column: isin_column.text.clone(),
//         csv_content: csv_content.to_string(),
//     };
//     Template::render("select-tickers", context)
// }
