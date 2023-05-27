use sailfish::TemplateOnce;
use serde::{Deserialize, Deserializer, Serialize};

use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, TemplateOnce)]
#[template(path = "edit.stpl")]
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
    // ? pub update_time: u64,
}

fn tags_deserialize<'de, D>(deserializer: D) -> Result<BTreeSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    Ok(str_sequence
        .split(' ')
        .filter_map(|item| (!item.is_empty()).then_some(item.to_string()))
        .collect())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    pub tag_name: String,
    pub bookmarks_count: u64,
    // ? pub is_favorite: bool,
    // ? pub color: String
}

#[derive(TemplateOnce)]
#[template(path = "tags.stpl")]
pub struct Tags {
    pub tags: Vec<Tag>,
}

#[derive(TemplateOnce)]
#[template(path = "index.stpl")]
pub struct Index {
    pub bookmarks: Vec<Bookmark>,
    pub number: usize,
    pub pg: usize,
    pub pages: usize,
}
