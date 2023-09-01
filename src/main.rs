#![feature(iter_array_chunks)]
#![allow(clippy::let_and_return)]

mod database;
mod handlers;
mod types;

use axum::routing::{delete, get, post, put};
use minijinja::{path_loader, value::Value, Environment};
use minijinja_contrib::filters::datetimeformat;
use types::MyError;

use std::sync::{Arc, Mutex};

use handlers::*;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<database::Db>>,
    pub env: minijinja::Environment<'static>,
}

impl AppState {
    pub fn render(&self, name: &str, ctx: Value) -> Result<String, MyError> {
        Ok(self.env.get_template(name)?.render(ctx)?)
    }
}

#[tokio::main]
async fn main() {
    let mut env = Environment::new();
    env.set_loader(path_loader("templates"));
    env.add_filter("datetimeformat", datetimeformat);

    let state = AppState {
        db: Arc::new(Mutex::new(database::Db::new(
            &std::env::args().nth(1).unwrap_or("main.db3".to_string()),
        ))),
        env,
    };
    println!("Server is starting at http://localhost:3000");

    let app = axum::Router::new()
        .route("/", get(index))
        .route("/all", get(page))
        .route("/tags", get(tags_page))
        .route("/tags/:name", get(tag_page))
        .route("/search", get(search))
        .route("/add-bookmarks", post(add_bookmarks_form))
        .route("/edit-bookmark/:id", post(update_bookmark_form))
        .route("/edit-bookmark/:id", get(edit_page))
        .route("/delete-bookmark", delete(delete_bookmark))
        .route("/set-tag/:id/:tag", put(set_tag))
        .route("/rename-tag/:old", post(rename_tag))
        .route("/delete-tag/:name", delete(delete_tag))
        .route("/set-favorite/:name", put(set_favorite))
        .route("/export-csv", get(export_csv))
        .route("/all-tags", get(all_tags))
        .with_state(state);

    axum::Server::bind(&std::net::SocketAddr::from(([127, 0, 0, 1], 3000)))
        .serve(app.into_make_service())
        .await
        .expect("Can't start server!");
}
