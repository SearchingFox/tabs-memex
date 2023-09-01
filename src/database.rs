use rusqlite::{
    config::DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, named_params, params, Connection, OpenFlags,
    Result,
};

use std::collections::BTreeSet;

use crate::types::{Bookmark, Page, Tag};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new(file_path: &str) -> Self {
        match Connection::open_with_flags(
            file_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(conn) => Db { conn },
            Err(_) => {
                let conn = Connection::open(file_path)
                    .expect("Error while opening connection to database");
                conn.set_db_config(SQLITE_DBCONFIG_ENABLE_FKEY, true)
                    .unwrap();
                conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS bookmarks (
                    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                    url             TEXT NOT NULL CHECK(url <> '') UNIQUE,
                    name            TEXT NOT NULL,
                    creation_time   INTEGER NOT NULL,
                    description     TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS tags (
                    tag_name       TEXT NOT NULL,
                    bookmark_id    INTEGER NOT NULL,
                    FOREIGN KEY(bookmark_id) REFERENCES bookmarks(id) ON DELETE CASCADE,
                    UNIQUE(tag_name, bookmark_id) ON CONFLICT IGNORE,
                    CHECK(tag_name <> '') ON CONFLICT IGNORE
                );

                CREATE TABLE IF NOT EXISTS favorites(
                    path TEXT NOT NULL,
                    UNIQUE (path) ON CONFLICT IGNORE
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
                END;

                CREATE TRIGGER tags_done_ai AFTER INSERT ON tags WHEN new.tag_name = 'done' BEGIN
                    DELETE FROM tags WHERE tag_name = 'todo' AND bookmark_id = new.bookmark_id;
                END;").expect("Execution of initial sql sequence failed");

                Db { conn }
            }
        }
    }

    pub fn update_bookmark(&self, new: Bookmark) -> Result<Bookmark> {
        self.conn.execute(
            "UPDATE bookmarks SET name = :new_name, url = :new_url, description = :new_description WHERE id = :id",
            named_params! {
                ":id": new.id,
                ":new_name": new.name,
                ":new_url": new.url,
                ":new_description": new.description,
            },
        )?;

        self.conn
            .execute("DELETE FROM tags WHERE bookmark_id = ?", params![new.id])?;

        for tag in new.tags {
            self.conn.execute(
                "INSERT OR IGNORE INTO tags VALUES (?1, ?2)",
                params![tag.to_lowercase(), new.id],
            )?;
        }

        self.get_bookmark_by_id(new.id)
    }

    pub fn delete_bookmark(&self, ids: &[i64]) -> Result<Vec<Bookmark>> {
        let mut res: Vec<Bookmark> = Vec::new();
        for id in ids {
            res.push(self.get_bookmark_by_id(*id)?);

            self.conn
                .execute("DELETE FROM bookmarks WHERE id = ?", params![id])?;
            self.conn
                .execute("DELETE FROM tags WHERE bookmark_id = ?", params![id])?;
        }

        println!("Deleted {res:?}");
        Ok(res)
    }

    pub fn set_tag(&self, name: &str, id: i64) -> Result<Bookmark> {
        self.conn
            .execute("INSERT INTO tags VALUES (?1, ?2)", params![name, id])?;

        self.get_bookmark_by_id(id)
    }

    pub fn delete_tag(&self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM tags WHERE tag_name = ?", params![name])?;

        Ok(())
    }

    pub fn rename_tag(&self, old: &str, new: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tags SET tag_name = ?1 WHERE tag_name = ?2",
            params![new, old],
        )?;

        Ok(())
    }

    pub fn set_favorite(&self, path: &str) -> Result<()> {
        self.conn
            .execute("INSERT INTO favorites VALUES ?", params![path])?;

        Ok(())
    }

    pub fn get_favorites(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT * FROM favorites")?;

        let res = stmt.query_map([], |row| row.get("path"))?.collect();

        res
    }

    pub fn list_tags(&self) -> Result<Box<[Tag]>> {
        let res = self
            .conn
            .prepare("SELECT tag_name, count(bookmark_id) FROM tags GROUP BY tag_name")?
            .query_map([], |row| {
                Ok(Tag {
                    tag_name: row.get(0)?,
                    bookmarks_count: row.get(1)?,
                })
            })?
            .collect();

        res
    }

    pub fn get_page(&self, page: Page) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM bookmarks
            LEFT JOIN (SELECT group_concat(tag_name) AS tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id
            WHERE id IN (SELECT id FROM bookmarks ORDER BY creation_time DESC LIMIT ?1 OFFSET ?2)
                AND NOT EXISTS(
                    SELECT 1 FROM tags WHERE tag_name LIKE 'private%' AND bookmark_id = id)
            ORDER BY creation_time DESC",
        )?;
        let res = stmt
            .query_map(params![page.limit, page.offset * page.limit], |row| {
                Ok(Bookmark {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    url: row.get("url")?,
                    creation_time: row.get("creation_time")?,
                    description: row.get("description")?,
                    tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                        x.split(',').map(String::from).collect()
                    }),
                })
            })?
            .collect();

        res
    }

    pub fn get_bookmark_by_id(&self, id: i64) -> Result<Bookmark> {
        self.conn.query_row(
            "SELECT * FROM bookmarks
            LEFT JOIN (SELECT group_concat(tag_name) AS tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id WHERE id = ?",
            params![id],
            |row| {
                Ok(Bookmark {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    url: row.get("url")?,
                    creation_time: row.get("creation_time")?,
                    description: row.get("description")?,
                    tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                        x.split(',').map(String::from).collect()
                    }),
                })
            },
        )
    }

    pub fn get_bookmark_by_url(&self, url: &str) -> Result<Bookmark> {
        self.conn.query_row(
            "SELECT * FROM bookmarks
            LEFT JOIN (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id WHERE url = ?",
            params![url],
            |row| {
                Ok(Bookmark {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    url: row.get("url")?,
                    creation_time: row.get("creation_time")?,
                    description: row.get("description")?,
                    tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                        x.split(',').map(String::from).collect()
                    }),
                })
            },
        )
    }

    pub fn get_bookmarks_by_tag(&self, tag_name: &str) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, url, creation_time, description, group_concat(tag_name) AS tags
            FROM bookmarks JOIN tags ON id = bookmark_id
            WHERE bookmark_id IN
                (SELECT id FROM bookmarks JOIN tags ON id = bookmark_id
                WHERE tag_name = ?1 AND CASE WHEN ?1 LIKE 'private%'
                    THEN 1
                    ELSE NOT EXISTS(SELECT 1 FROM tags WHERE tag_name LIKE 'private%' AND bookmark_id = id)
                    END)
            GROUP BY bookmark_id
            ORDER BY creation_time DESC",
        )?;
        let res = stmt
            .query_map(params![tag_name], |row| {
                Ok(Bookmark {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    url: row.get("url")?,
                    creation_time: row.get("creation_time")?,
                    description: row.get("description")?,
                    tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                        x.split(',').map(String::from).collect()
                    }),
                })
            })?
            .collect();

        res
    }

    pub fn get_bookmarks_by_date(&self, date: &str) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM bookmarks LEFT JOIN
                (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id
            WHERE date(creation_time, 'unixepoch', 'localtime') = ?
                AND NOT EXISTS(SELECT 1 FROM tags WHERE tag_name = 'private' AND bookmark_id = id)
            ORDER BY creation_time DESC",
        )?;
        let res = stmt
            .query_map(params![date], |row| {
                Ok(Bookmark {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    url: row.get("url")?,
                    creation_time: row.get("creation_time")?,
                    description: row.get("description")?,
                    tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                        x.split(',').map(String::from).collect()
                    }),
                })
            })?
            .collect();

        res
    }

    pub fn search(&self, query: &str) -> Result<Vec<Bookmark>> {
        if query.starts_with("# ") {
            let mut stmt = self.conn.prepare(
                format!(
                    "SELECT * FROM bookmarks LEFT JOIN
                        (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
                    ON id = bookmark_id
                    WHERE id IN (
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
                        id: row.get("id")?,
                        name: row.get("name")?,
                        url: row.get("url")?,
                        creation_time: row.get("creation_time")?,
                        description: row.get("description")?,
                        tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                            x.split(',').map(String::from).collect()
                        }),
                    })
                })?
                .collect::<Result<Vec<Bookmark>>>();

            return res;
        }

        let mut stmt = self.conn.prepare(
            "SELECT rowid, highlight(bookmarks_fts, 0, '<mark>', '</mark>') name, url, creation_time,
                highlight(bookmarks_fts, 3, '<mark>', '</mark>') description, tags
            FROM bookmarks_fts LEFT JOIN
                (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON rowid = bookmark_id
            WHERE bookmarks_fts MATCH ?
            ORDER BY creation_time DESC",
        )?;
        // bm25(bookmarks_fts)

        let mut res: Vec<Bookmark> = stmt
            .query_map(params![query], |row| {
                Ok(Bookmark {
                    id: row.get("rowid")?,
                    name: row.get("name")?,
                    url: row.get("url")?,
                    creation_time: row.get("creation_time")?,
                    description: row.get("description")?,
                    tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                        x.split(',').map(String::from).collect()
                    }),
                })
            })?
            .collect::<Result<Vec<Bookmark>>>()
            .unwrap_or(Vec::new());

        if res.is_empty() {
            stmt = self.conn.prepare(
                "SELECT * FROM bookmarks LEFT JOIN
                    (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
                ON id = bookmark_id
                WHERE url LIKE '%' || ?1 || '%'
                    OR name LIKE '%' || ?1 || '%'
                    OR description LIKE '%' || ?1 || '%'
                ORDER BY creation_time DESC",
            )?;

            res = stmt
                .query_map(params![query], |row| {
                    Ok(Bookmark {
                        id: row.get("id")?,
                        name: row.get("name")?,
                        url: row.get("url")?,
                        creation_time: row.get("creation_time")?,
                        description: row.get("description")?,
                        tags: row.get("tags").map_or(BTreeSet::new(), |x: String| {
                            x.split(',').map(String::from).collect()
                        }),
                    })
                })?
                .collect::<Result<Vec<Bookmark>>>()
                .unwrap_or(Vec::new());
        }

        Ok(res)
    }

    pub fn count_all(&self) -> Result<usize> {
        self.conn
            .query_row_and_then("SELECT count() FROM bookmarks", [], |row| row.get(0))
    }

    pub fn insert(&mut self, input: &str, all_tags: &str) -> Result<Vec<Bookmark>> {
        let tx = self.conn.transaction()?;
        let mut existing: Vec<String> = Vec::new();
        let mut not_existing: Vec<i64> = Vec::new();

        let bookmarks: Vec<Bookmark> = input
            .lines()
            .array_chunks()
            .map(|x: [&str; 2]| {
                let (url, tags) = x[1].split_once(' ').map_or(
                    (
                        x[1].to_string(),
                        all_tags.split(' ').map(String::from).collect(),
                    ),
                    |(url, tags)| {
                        (
                            url.to_string(),
                            tags.split(' ')
                                .chain(all_tags.split(' '))
                                .map(String::from)
                                .collect(),
                        )
                    },
                );
                let name = if x[0].is_empty() {
                    url.clone()
                } else {
                    x[0].to_string()
                };

                Bookmark {
                    name,
                    url,
                    tags,
                    ..Default::default()
                }
            })
            .collect();

        for new in bookmarks {
            if let Err(err) = tx.execute(
                "INSERT INTO bookmarks (name, url, creation_time, description) VALUES (?1, ?2, unixepoch(), '')",
                params![new.name.replace('<', "&lt").replace('>', "&gt"), new.url]
            ) {
                println!("{}: {}", err, new.url);

                for tag_name in new.tags {
                    tx.execute(
                        "INSERT OR IGNORE INTO tags SELECT ?1, id FROM bookmarks WHERE url = ?2",
                        params![tag_name.to_lowercase(), new.url])?;
                }

                existing.push(new.url);
            } else {
                let bookmark_id = tx.last_insert_rowid();
                for tag_name in new.tags {
                    tx.execute(
                        "INSERT OR IGNORE INTO tags (tag_name, bookmark_id) VALUES (?1, ?2)",
                        params![tag_name.to_lowercase(), bookmark_id,],
                    )?;
                }

                not_existing.push(bookmark_id);
            }
        }

        println!("Inserted {}", not_existing.len());
        tx.commit()?;

        existing
            .iter()
            .map(|x| -> Result<Bookmark> {
                let mut b = self.get_bookmark_by_url(x)?;
                b.tags.insert("dup".to_string());
                Ok(b)
            })
            .chain(not_existing.into_iter().map(|x| self.get_bookmark_by_id(x)))
            .collect()
    }

    pub fn export_csv(&self) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM bookmarks
                LEFT JOIN (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id")?;
        let mut data = vec!["title,note,excerpt,url,tags,created,cover,highlights".to_string()];

        for row in stmt.query_map([], |row| {
            Ok([
                row.get("name")?,
                format!("\"{}\"", row.get::<&str, String>("description")?),
                "".to_string(),
                row.get("url")?,
                row.get("tags").unwrap_or_default(),
                row.get("creation_time")?,
                "".to_string(),
                "".to_string(),
            ]
            .join(","))
        })? {
            data.push(row?)
        }

        Ok(data.join("\n"))
    }
}

impl Default for Db {
    fn default() -> Self {
        Self::new("./main.db3")
    }
}
