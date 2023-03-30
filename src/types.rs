use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
pub struct Bookmark {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub creation_time: u64, // maybe use string with ISO 8601
    pub tags: BTreeSet<String>,
    // ? update_time
    // ? description: String, use for youtube timestamp
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    pub tag_name: String,
    pub bookmarks_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddUrlsForm {
    pub urls: String,
}

#[derive(Deserialize)]
pub struct EditBookmarkForm {
    pub url: String,
    pub name: String,
    pub tags: String,
}

#[derive(Deserialize, Default)]
pub struct Page {
    pub p: Option<u64>,
}

#[derive(Deserialize)]
pub struct Search {
    pub q: String,
}
