use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::{Deserialize, Deserializer, Serialize};

use std::collections::BTreeSet;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Bookmark {
    #[serde(skip_deserializing)]
    pub id: i64,
    pub name: String,
    pub url: String,
    #[serde(skip_deserializing)]
    pub creation_time: i64,
    pub description: String,
    #[serde(deserialize_with = "tags_deserialize")]
    pub tags: BTreeSet<String>,
}

fn tags_deserialize<'de, D>(deserializer: D) -> Result<BTreeSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(String::deserialize(deserializer)?
        .split(' ')
        .map(String::from)
        .collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    pub tag_name: String,
    pub bookmarks_count: u64,
}

#[derive(Deserialize)]
pub struct Page {
    pub p: Option<usize>,
    pub limit: Option<usize>,
    pub sort: Option<String>,
}

pub struct MyError(pub String);

impl<E: std::error::Error> From<E> for MyError {
    fn from(err: E) -> Self {
        MyError(format!("Error: {err}"))
    }
}

impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        let MyError(body) = self;
        (StatusCode::INTERNAL_SERVER_ERROR, Html(body)).into_response()
    }
}
