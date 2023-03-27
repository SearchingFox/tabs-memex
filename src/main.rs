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
    get, middleware, post,
    web::{self, Form, Path, Redirect},
    App, HttpResponse, HttpServer, Responder, Result,
};
use futures_util::StreamExt as _;
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

mod database;
mod templates;

#[derive(Clone, Debug, Serialize)]
pub struct Bookmark {
    id: u64,
    name: String,
    url: String,
    creation_time: u64, // maybe use string with ISO 8601
    tags: BTreeSet<String>,
    // ? update_time
    // ? description: String, use for youtube timestamp
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    tag_name: String,
    bookmarks_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct AddUrlsForm {
    urls: String,
}

#[post("/add-urls")]
async fn add_urls_form(Form(form): Form<AddUrlsForm>) -> Result<impl Responder> {
    database::insert_from_lines(form.urls).map_err(ErrorInternalServerError)?;
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

#[derive(Deserialize)]
struct EditBookmarkForm {
    url: String,
    name: String,
    tags: String,
}

#[post("/update-bookmark/{id}")]
async fn update_bookmark_form(
    id: Path<u64>,
    form: Form<EditBookmarkForm>,
) -> Result<impl Responder> {
    database::update_bookmark(Bookmark {
        id: id.into_inner(),
        name: form.name.clone(),
        url: form.url.clone(),
        creation_time: 0,
        tags: form
            .tags
            .split(' ')
            .map(String::from)
            .collect::<BTreeSet<_>>(),
    })
    .map_err(ErrorInternalServerError)?;
    Ok(Redirect::to("/").see_other())
}

#[get("edit-bookmark/{id}/")]
async fn edit_page(id: Path<u64>) -> Result<HttpResponse> {
    templates::edit_page(
        database::get_bookmark_by_id(id.into_inner()).map_err(ErrorInternalServerError)?,
    )
    .map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("delete-bookmark/{id}/")]
async fn delete_bookmark(id: Path<u64>) -> Result<impl Responder> {
    database::delete_bookmark(id.into_inner()).map_err(ErrorInternalServerError)?;
    Ok(Redirect::to("/").see_other())
}

#[get("tags/")]
async fn tags_page() -> Result<HttpResponse> {
    templates::tags_page(database::list_tags().map_err(ErrorInternalServerError)?).map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("tags/{name}/")]
async fn tag_page(name: Path<String>) -> Result<HttpResponse> {
    templates::index_page(
        database::get_bookmarks_by_tag(name.into_inner()).map_err(ErrorInternalServerError)?,
    )
    .map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("date/{date}/")]
async fn date_page(date: Path<String>) -> Result<HttpResponse> {
    templates::index_page(
        database::get_bookmarks_by_date(date.into_inner()).map_err(ErrorInternalServerError)?,
    )
    .map_or_else(
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
    println!("Server started at http://localhost:1739");
    HttpServer::new(|| {
        App::new()
            // .wrap(middleware::NormalizePath::trim())
            .app_data(web::FormConfig::default().limit(4096))
            .service(tag_page)
            .service(date_page)
            .service(home_page)
            .service(tags_page)
            .service(edit_page)
            .service(delete_bookmark)
            .service(add_file_form)
            .service(add_urls_form)
            .service(update_bookmark_form)
    })
    .bind(("127.0.0.1", 1739))?
    .run()
    .await
}
