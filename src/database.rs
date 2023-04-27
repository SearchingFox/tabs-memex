use rusqlite::{named_params, params, Connection, Result};

use std::{collections::BTreeSet, path::Path};

use crate::types::{Bookmark, Tag};

const FILE_PATH: &str = "./my_db.db3";

pub fn init() -> Result<()> {
    if !Path::new(FILE_PATH).exists() {
        let conn = Connection::open(FILE_PATH)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS bookmarks (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                url            TEXT NOT NULL CHECK (url <> '') UNIQUE,
                name           TEXT NOT NULL,
                creation_time  INTEGER NOT NULL,
                description    TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tags (
                tag_name       TEXT NOT NULL CHECK (tag_name <> ''),
                bookmark_id    INTEGER NOT NULL,
                UNIQUE (tag_name, bookmark_id) ON CONFLICT IGNORE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS bookmarks_fts
                USING fts5(name, url, creation_time UNINDEXED, description, content='bookmarks', content_rowid='id', tokenize='trigram');
            CREATE TRIGGER bookmarks_ai AFTER INSERT ON bookmarks BEGIN
                INSERT INTO bookmarks_fts(rowid, name, url, creation_time, description) VALUES (new.id, new.name, new.url, new.creation_time, new.description);
            END;
            CREATE TRIGGER bookmarks_ad AFTER DELETE ON bookmarks BEGIN
                INSERT INTO bookmarks_fts(bookmarks_fts, rowid, name, url, creation_time, description) VALUES('delete', old.id, old.name, old.url, old.creation_time, old.description);
            END;
            CREATE TRIGGER bookmarks_au AFTER UPDATE ON bookmarks BEGIN
                INSERT INTO bookmarks_fts(bookmarks_fts, rowid, name, url, creation_time, description) VALUES('delete', old.id, old.name, old.url, old.creation_time, old.description);
                INSERT INTO bookmarks_fts(rowid, name, url, creation_time, description) VALUES (new.id, new.name, new.url, new.creation_time, new.description);
            END;")?;
    }

    Ok(())
}

pub fn insert(bookmarks: Vec<Bookmark>) -> Result<()> {
    let mut conn = Connection::open(FILE_PATH)?;
    let tx = conn.transaction()?;

    for b in bookmarks {
        if let Err(err) = tx.execute(
            "INSERT INTO bookmarks (name, url, creation_time, description) VALUES (?1, ?2, unixepoch(), '')",
            params![
                b.name.replace('<', "&lt").replace('>', "&gt"),
                b.url,
            ],
        ) {
            println!("{}: {}", err, b.url)
        }
    }

    tx.commit()
}

pub fn update_bookmark(b: Bookmark) -> Result<()> {
    let conn = Connection::open(FILE_PATH)?;

    conn.execute(
        "UPDATE bookmarks SET name = :new_name, url = :new_url, description = :new_description WHERE id = :id",
        named_params! {
            ":id": b.id,
            ":new_name": b.name,
            ":new_url": b.url,
            ":new_description": b.description,
        },
    )?;

    conn.execute("DELETE FROM tags WHERE bookmark_id = ?", params![b.id])?;

    if !b.tags.is_empty() {
        for tag in b.tags {
            if !tag.is_empty() {
                conn.execute("INSERT INTO tags VALUES (?1, ?2)", params![tag, b.id])?;
            }
        }
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
        .query_map(named_params! {":id": id.to_string()}, |row| row.get(0))?
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

pub fn list_all(page: i32) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, url, creation_time, description \
        FROM bookmarks \
        WHERE NOT EXISTS( \
            SELECT 1 FROM tags WHERE tag_name = 'private' AND bookmark_id = id) \
        ORDER BY creation_time DESC \
        LIMIT ?1 OFFSET ?2",
    )?;
    let res = stmt
        .query_map(params![100, page * 100], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                creation_time: row.get(3)?,
                description: row.get(4)?,
                tags: tags_for_bookmark(row.get(0)?)?,
            })
        })?
        .collect();

    res
}

pub fn get_bookmark_by_id(id: u64) -> Result<Bookmark> {
    let conn = Connection::open(FILE_PATH)?;

    conn.query_row(
        "SELECT id, name, url, creation_time, description FROM bookmarks WHERE id = ?",
        params![id],
        |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                creation_time: row.get(3)?,
                description: row.get(4)?,
                tags: tags_for_bookmark(id)?,
            })
        },
    )
}

pub fn get_bookmarks_by_tag(tag_name: String) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, url, creation_time, description
        FROM bookmarks JOIN tags ON id = bookmark_id WHERE tag_name = ?
        ORDER BY creation_time DESC",
    )?;
    let res = stmt
        .query_map(params![tag_name], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                creation_time: row.get(3)?,
                description: row.get(4)?,
                tags: tags_for_bookmark(row.get(0)?)?,
            })
        })?
        .collect();

    res
}

pub fn get_bookmarks_by_date(date: &String) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, url, creation_time, description
        FROM bookmarks WHERE date(creation_time, 'unixepoch', 'localtime') = ?
        ORDER BY creation_time DESC",
    )?;
    let res = stmt
        .query_map(params![date], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                creation_time: row.get(3)?,
                description: row.get(4)?,
                tags: tags_for_bookmark(row.get(0)?)?,
            })
        })?
        .collect();

    res
}

pub fn search(query: &String) -> Result<Vec<Bookmark>> {
    let conn = Connection::open(FILE_PATH)?;

    if query.starts_with("tags:") {
        let mut stmt = conn.prepare(
            format!(
                "SELECT id, name, url, creation_time, description FROM bookmarks WHERE id IN (
                {}
                ) ORDER BY creation_time DESC",
                query
                    .split(' ')
                    .skip(1)
                    .map(|s| format!("SELECT bookmark_id FROM tags WHERE tag_name = '{}'", s))
                    .collect::<Vec<String>>()
                    .join(" INTERSECT ")
            )
            .as_str(),
        )?;

        let res = stmt
            .query_map(params![], |row| {
                Ok(Bookmark {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    creation_time: row.get(3)?,
                    description: row.get(4)?,
                    tags: tags_for_bookmark(row.get(0)?)?,
                })
            })?
            .collect::<Result<Vec<Bookmark>>>();

        return res;
    }

    let mut stmt = conn.prepare(
        "SELECT rowid, highlight(bookmarks_fts, 0, '<mark>', '</mark>') name, url, creation_time, highlight(bookmarks_fts, 3, '<mark>', '</mark>') description
         FROM bookmarks_fts WHERE bookmarks_fts MATCH ?
         ORDER BY bm25(bookmarks_fts)",
    )?;

    let mut res: Vec<Bookmark> = stmt
        .query_map(params![query], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                creation_time: row.get(3)?,
                description: row.get(4)?,
                tags: tags_for_bookmark(row.get(0)?)?,
            })
        })?
        .collect::<Result<Vec<Bookmark>>>()
        .unwrap_or(Vec::new());

    if res.is_empty() {
        stmt = conn.prepare(
            "SELECT id, name, url, creation_time, description
             FROM bookmarks WHERE
             url LIKE '%' || ?1 || '%' OR name LIKE '%' || ?1 || '%' OR description LIKE '%' || ?1 || '%'
             ORDER BY creation_time DESC",
        )?;

        res = stmt
            .query_map(params![query], |row| {
                Ok(Bookmark {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    creation_time: row.get(3)?,
                    description: row.get(4)?,
                    tags: tags_for_bookmark(row.get(0)?)?,
                })
            })?
            .collect::<Result<Vec<Bookmark>>>()
            .unwrap_or(Vec::new());
    }

    Ok(res)
}

pub fn count_all() -> Result<i32> {
    let conn = Connection::open(FILE_PATH)?;

    conn.query_row_and_then("SELECT count() FROM bookmarks", [], |row| row.get(0))
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
                creation_time: 0,
                description: "".to_string(),
                tags: BTreeSet::new(),
            })
            .collect(),
    )
}
