use serde::{Deserialize, Deserializer, Serialize};

use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bookmark {
    #[serde(skip_deserializing)]
    pub id: u64,
    pub name: String,
    pub url: String,
    #[serde(skip_deserializing)]
    pub creation_time: u64, // maybe use string with ISO 8601
    #[serde(deserialize_with = "tags_deserialize")]
    pub tags: BTreeSet<String>,
    // pub description: String,
    // ? pub update_time: u64,
}

fn tags_deserialize<'de, D>(deserializer: D) -> Result<BTreeSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    Ok(str_sequence
        .split(' ')
        .map(|item| item.to_string())
        .collect())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    pub tag_name: String,
    pub bookmarks_count: u64,
}
