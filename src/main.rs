#![feature(iter_array_chunks)]
#![feature(file_create_new)]

#[macro_use]
extern crate tera;
#[macro_use]
extern crate lazy_static;
extern crate serde_json;

use actix_web::{web, App, HttpServer};

mod database;
mod handlers;
mod templates;
mod types;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    database::init().expect("Database initialization failed");
    println!("Server started at http://localhost:1739");
    HttpServer::new(|| {
        App::new()
            .app_data(web::FormConfig::default().limit(131_072))
            .service(handlers::page)
            .service(handlers::tag_page)
            .service(handlers::date_page)
            .service(handlers::tags_page)
            .service(handlers::edit_page)
            .service(handlers::delete_bookmark)
            .service(handlers::search)
            .service(handlers::add_file_form)
            .service(handlers::add_urls_form)
            .service(handlers::update_bookmark_form)
    })
    .bind(("127.0.0.1", 1739))?
    .run()
    .await
}
