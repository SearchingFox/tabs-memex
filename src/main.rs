#![feature(iter_array_chunks)]

use actix_web::{web, App, HttpServer};

mod database;
mod handlers;
mod types;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    database::init().expect("Database initialization failed");
    println!("Server started at http://localhost:1739");
    HttpServer::new(|| {
        App::new()
            // .wrap(NormalizePath::trim())
            .app_data(web::FormConfig::default().limit(128 * 1024))
            .service(handlers::page)
            .service(handlers::tag_page)
            .service(handlers::tags_page)
            .service(handlers::edit_page)
            .service(handlers::delete_bookmark)
            .service(handlers::search)
            .service(handlers::add_file_form)
            .service(handlers::add_urls_form)
            .service(handlers::update_bookmark_form)
            .service(handlers::style)
    })
    .bind(("127.0.0.1", 1739))?
    .run()
    .await
}
