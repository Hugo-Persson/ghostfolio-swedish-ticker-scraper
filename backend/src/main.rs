#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::serde::{json::Json, Deserialize};
use rocket::Request;
use rocket_dyn_templates::Template;
use serde::Serialize;
use serde_json::json;

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
    isin_column: bool,
    file: TempFile<'r>,
}

#[post("/init", data = "<data>")]
fn init_form(data: Form<InitTickersFormData<'_>>) -> &'static str {
    println!("{}", data.isin_column);
    return "Success";
}
