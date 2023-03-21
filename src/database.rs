use rusqlite::{Connection, Result};

use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Bookmark, Tag};

const FILE_PATH: &str = "./my_db.db3";

pub fn init() -> Result<()> {
    match File::create_new(FILE_PATH) {
        Ok(_) => println!("Created new database file"),
        _ => {}
    };
    let conn = Connection::open(FILE_PATH)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bookmarks (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            url            TEXT NOT NULL CHECK (url <> ''),
            name           TEXT NOT NULL,
            creation_time  INTEGER NOT NULL
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            tag_name       TEXT NOT NULL,
            bookmark_id    INTEGER PRIMARY KEY
        )",
        (),
    )?;

    Ok(())
}

pub fn insert(inp: Vec<Bookmark>) -> Result<()> {
    let conn = Connection::open(FILE_PATH)?;

    for b in inp {
        conn.execute(
            "INSERT INTO bookmarks (name, url, creation_time) VALUES (?1, ?2, ?3)",
            (&b.name, &b.url, &b.creation_time.to_string()),
        )?;
    }

    Ok(())
}

pub fn tags_for_bookmark(id: i32) -> Result<Vec<Tag>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare("SELECT DISTINCT tag_name FROM tags WHERE bookmark_id = :id")?;
    let res = stmt
        .query_map([":id", id.to_string().as_str()], |row| {
            Ok(Tag {
                tag_name: row.get(0)?,
            })
        })?
        .collect();

    res
}

pub fn list_tags() -> Result<Vec<Tag>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare("SELECT tag_name FROM tags")?;
    let res = stmt
        .query_map([], |row| {
            Ok(Tag {
                tag_name: row.get(0)?,
            })
        })?
        .collect();

    res
}

pub fn list_all() -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, url, creation_time FROM bookmarks ORDER BY creation_time DESC",
    )?;
    let res = stmt
        .query_map([], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                creation_time: row.get(3)?,
            })
        })?
        .collect();

    res
}

pub fn insert_from_lines(input: String) -> Result<()> {
    insert(
        input
            .lines()
            .array_chunks()
            .map(|x: [&str; 2]| Bookmark {
                id: 0,
                name: x[0].to_string(),
                url: x[1].to_string(),
                creation_time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs(),
            })
            .collect(),
    )
}
