use rusqlite::{Connection, Result};

use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Bookmark, Tag};

const FILE_PATH: &str = "./my_db.db3";

pub fn init() -> Result<()> {
    if File::create_new(FILE_PATH).is_ok() {
        println!("Created new database file")
    }
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
            bookmark_id    INTEGER NOT NULL,
            UNIQUE (tag_name, bookmark_id) ON CONFLICT IGNORE
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

pub fn update_bookmark(
    id: u64,
    new_url: String,
    new_name: String,
    new_tags: Vec<Tag>,
) -> Result<()> {
    // conn: &Connection
    let conn = Connection::open(FILE_PATH)?;

    conn.execute(
        "UPDATE bookmarks SET name = :new_name, url = :new_url WHERE id = :id",
        &[
            (":id", &id.to_string()),
            (":new_name", &new_name),
            (":new_url", &new_url),
        ],
    )?;

    for nt in new_tags {
        conn.execute(
            "INSERT INTO tags VALUES (?1, ?2)",
            (&nt.tag_name, &id.to_string()),
        )?;
    }

    Ok(())
}

pub fn tags_for_bookmark(id: u64) -> Result<Vec<Tag>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare("SELECT DISTINCT tag_name FROM tags WHERE bookmark_id = :id")?;
    let res = stmt
        .query_map(&[(":id", &id.to_string())], |row| {
            Ok(Tag {
                tag_name: row.get(0)?,
                bookmarks_count: 0,
            })
        })?
        .collect();

    res
}

pub fn list_tags() -> Result<Vec<Tag>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt =
        conn.prepare("SELECT tag_name, COUNT(bookmark_id) FROM tags GROUP BY tag_name")?;
    let res = stmt
        .query_map([], |row| {
            Ok(Tag {
                tag_name: row.get(0)?,
                bookmarks_count: row.get(1)?,
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
                tags: tags_for_bookmark(row.get(0)?)?,
            })
        })?
        .collect();

    res
}

pub fn get_bookmark_by_id(id: u64) -> Result<Bookmark> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt =
        conn.prepare("SELECT id, name, url, creation_time FROM bookmarks WHERE id = :id")?;
    let res: Result<Vec<Bookmark>> = stmt
        .query_map(&[(":id", &id.to_string())], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                creation_time: row.get(3)?,
                tags: tags_for_bookmark(id)?,
            })
        })?
        .collect();

    res.map(|x| x[0].clone())
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
                tags: Vec::new(),
            })
            .collect(),
    )
}
