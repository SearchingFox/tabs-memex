use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::{Deserialize, Deserializer, Serialize};

use std::collections::BTreeSet;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Bookmark {
    #[serde(skip_deserializing)]
    pub id: i64,
    pub name: String,
    pub url: String,
    #[serde(skip_deserializing)]
    pub creation_time: i64, // maybe use string with ISO 8601
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

pub struct Page {
    pub offset: usize,
    pub limit: usize,
}

pub struct MyError(pub String);

// E: Error doesn't work...
impl<D: std::fmt::Display> From<D> for MyError {
    fn from(err: D) -> Self {
        MyError(format!("Error: {}", err))
    }
}

impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        let MyError(body) = self;

        // https://github.com/bigskysoftware/htmx/issues/1619
        (StatusCode::OK, Html(body)).into_response()
    }
}
