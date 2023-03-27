use rusqlite::{params, Connection, Result};

use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{Bookmark, Tag};

const FILE_PATH: &str = "./my_db.db3";

pub fn init() -> Result<()> {
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

pub fn insert(bookmarks: Vec<Bookmark>) -> Result<()> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt =
        conn.prepare("INSERT INTO bookmarks (name, url, creation_time) VALUES (?1, ?2, ?3)")?;
    for b in bookmarks {
        stmt.execute(params![b.name, b.url, b.creation_time])?;
    }

    Ok(())
}

pub fn update_bookmark(b: Bookmark) -> Result<()> {
    let conn = Connection::open(FILE_PATH)?;

    conn.execute(
        "UPDATE bookmarks SET name = :new_name, url = :new_url WHERE id = :id",
        &[
            (":id", &b.id.to_string()),
            (":new_name", &b.name),
            (":new_url", &b.url),
        ],
    )?;

    for tag in tags_for_bookmark(b.id)?.difference(&b.tags) {
        conn.execute(
            "DELETE FROM tags WHERE tag_name = ?1 AND bookmark_id = ?2",
            params![tag, b.id],
        )?;
    }

    for new_tag in b.tags.difference(&tags_for_bookmark(b.id)?) {
        conn.execute("INSERT INTO tags VALUES (?1, ?2)", params![new_tag, b.id])?;
    }

    Ok(())
}

pub fn delete_bookmark(id: u64) -> Result<()> {
    let conn = Connection::open(FILE_PATH)?;

    conn.execute("DELETE FROM bookmarks WHERE id = ?", params![id])?;
    conn.execute("DELETE FROM tags WHERE bookmark_id = ?", params![id])?;

    Ok(())
}

pub fn tags_for_bookmark(id: u64) -> Result<BTreeSet<String>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare("SELECT DISTINCT tag_name FROM tags WHERE bookmark_id = :id")?;
    let res = stmt
        .query_map(&[(":id", &id.to_string())], |row| Ok(row.get(0)?))?
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
    stmt.query_row(&[(":id", &id.to_string())], |row| {
        Ok(Bookmark {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            creation_time: row.get(3)?,
            tags: tags_for_bookmark(id)?,
        })
    })
}

pub fn get_bookmarks_by_tag(tag_name: String) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, url, creation_time FROM bookmarks JOIN tags ON id = bookmark_id WHERE tag_name = :tag_name",
    )?;
    let res = stmt
        .query_map(&[(":tag_name", &tag_name)], |row| {
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

pub fn get_bookmarks_by_date(date: String) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, url, creation_time FROM bookmarks WHERE DATE(creation_time, 'unixepoch', 'utc') = :date",
    )?;
    let res = stmt
        .query_map(&[(":date", &date)], |row| {
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
                tags: BTreeSet::new(),
            })
            .collect(),
    )
}
