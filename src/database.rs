use rusqlite::{named_params, params, Connection, Result};

use std::{
    collections::BTreeSet,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::types::{Bookmark, Tag};

const FILE_PATH: &str = "./my_db.db3";

pub fn init() -> Result<()> {
    if !Path::new(FILE_PATH).exists() {
        let conn = Connection::open(FILE_PATH)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS bookmarks (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            url            TEXT NOT NULL CHECK (url <> '') UNIQUE,
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

        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS fts
             USING fts5(name, url, creation_time UNINDEXED, content='bookmarks', content_rowid='id')",
            (),
        )?;

        conn.execute(
            "CREATE TRIGGER bookmarks_ai AFTER INSERT ON bookmarks BEGIN
                INSERT INTO fts(rowid, name, url) VALUES (new.id, new.name, new.url);
             END;
            CREATE TRIGGER bookmarks_ad AFTER DELETE ON bookmarks BEGIN
                INSERT INTO fts(fts, rowid, name, url) VALUES('delete', old.id, old.name, old.url);
            END;
            CREATE TRIGGER bookmarks_au AFTER UPDATE ON bookmarks BEGIN
                INSERT INTO fts(fts, rowid, name, url) VALUES('delete', old.id, old.name, old.url);
                INSERT INTO fts(fts, rowid, name, url) VALUES (new.id, new.name, new.url);
            END;",
            (),
        )?;
    };

    Ok(())
}

pub fn insert(bookmarks: Vec<Bookmark>) -> Result<()> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt =
        conn.prepare("INSERT INTO bookmarks (name, url, creation_time) VALUES (?1, ?2, ?3)")?;
    for b in bookmarks {
        match stmt.execute(params![b.name, b.url, b.creation_time]) {
            Err(err) => println!("{}: {}", err, b.url),
            _ => {}
        }
    }

    Ok(())
}

pub fn update_bookmark(b: Bookmark) -> Result<()> {
    let conn = Connection::open(FILE_PATH)?;

    conn.execute(
        "UPDATE bookmarks SET name = :new_name, url = :new_url WHERE id = :id",
        named_params! {
            ":id": b.id,
            ":new_name": b.name,
            ":new_url": b.url,
        },
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
        .query_map(named_params! {":id": id.to_string()}, |row| Ok(row.get(0)?))?
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

pub fn list_all(page: u64) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, url, creation_time FROM bookmarks ORDER BY creation_time DESC LIMIT 50 OFFSET ?",
    )?;
    let res = stmt
        .query_map(params![page * 50], |row| {
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
        conn.prepare("SELECT id, name, url, creation_time FROM bookmarks WHERE id = ?")?;
    stmt.query_row(params![id], |row| {
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
        "SELECT id, name, url, creation_time FROM bookmarks JOIN tags ON id = bookmark_id WHERE tag_name = ?",
    )?;
    let res = stmt
        .query_map(params![tag_name], |row| {
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
        "SELECT id, name, url, creation_time FROM bookmarks WHERE DATE(creation_time, 'unixepoch', 'utc') = ?",
    )?;
    let res = stmt
        .query_map(params![date], |row| {
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

pub fn search(query: String) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT rowid, highlight(fts, 0, '<mark>', '</mark>') name, url, creation_time
         FROM fts WHERE fts MATCH ?
         ORDER BY creation_time DESC",
    )?;

    let res = stmt
        .query_map(params![query], |row| {
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
