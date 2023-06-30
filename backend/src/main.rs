#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::{response::content::Json, Request};
use rocket_contrib::templates::Template;
use serde::Serialize;

fn main() {
    rocket::ignite()
        .register(catchers![not_found])
        .mount("/api", routes![hello])
        .mount("/", routes![index, init])
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
    Template::render("init", {})
}
