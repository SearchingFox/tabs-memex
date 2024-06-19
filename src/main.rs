#![feature(iter_array_chunks)]
#![allow(clippy::let_and_return)]

mod database;
mod handlers;
mod types;

use axum::routing::{delete, get, post, put};
use minijinja::{path_loader, value::Value, Environment};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use tokio::net::TcpListener;

use std::sync::{Arc, Mutex};

use handlers::*;
use types::MyError;

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

fn datetimeformat(value: String) -> String {
    OffsetDateTime::from_unix_timestamp(value.parse().unwrap_or_default())
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
        .format(&Rfc3339)
        .map(|x| x[..16].to_string())
        .unwrap_or_default()
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
    println!("Server is running at http://localhost:3000");

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

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .await
        .expect("Can't start server!");
}
