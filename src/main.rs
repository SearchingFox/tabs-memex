#![feature(iter_array_chunks)]
#![feature(file_create_new)]

#[macro_use]
extern crate tera;
#[macro_use]
extern crate lazy_static;
extern crate serde_json;

use actix_multipart::Multipart;
use actix_web::{
    error::ErrorInternalServerError,
    get, post,
    web::{Form, Redirect},
    App, HttpResponse, HttpServer, Responder, Result,
};
use futures_util::StreamExt as _;
use serde::{Deserialize, Serialize};

mod database;
mod templates;

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    id: u64,
    name: String,
    url: String,
    creation_time: u64, // maybe use string with ISO 8601
                        // tags: Vec<String>,
                        // comments: String, use for youtube timestamp
                        // content wget or Path to html file or Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    tag_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddUrlsForm {
    urls: String,
}

#[post("/add-urls")]
async fn add_urls_form(params: Form<AddUrlsForm>) -> Result<impl Responder> {
    database::insert_from_lines(params.urls.clone()).map_err(ErrorInternalServerError)?;
    Ok(Redirect::to("/").see_other())
}

#[post("/")]
async fn add_file_form(mut payload: Multipart) -> Result<impl Responder> {
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let mut res = "".to_string();

        while let Some(chunk) = field.next().await {
            res.push_str(std::str::from_utf8(&chunk?)?);
        }

        database::insert_from_lines(res).map_err(ErrorInternalServerError)?;
    }

    Ok(Redirect::to("/").see_other())
}

#[get("edit-link/{id}")]
async fn edit_link() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body(""))
}

#[get("cat/")]
async fn tags_page() -> Result<HttpResponse> {
    templates::tags_page(database::list_tags().map_err(ErrorInternalServerError)?).map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("/")]
async fn home_page() -> Result<HttpResponse> {
    templates::index_page(database::list_all().map_err(ErrorInternalServerError)?).map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    database::init().expect("Database initialization failed");
    println!("Server started at http://localhost:1738");
    HttpServer::new(|| {
        App::new()
            .service(home_page)
            .service(tags_page)
            .service(edit_link)
            .service(add_file_form)
            .service(add_urls_form)
    })
    .bind(("127.0.0.1", 1738))?
    .run()
    .await
}
