use rusqlite::{
    Connection, OpenFlags, Result, config::DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, params,
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
                    .expect("Error while opening connection to new database");
                conn.set_db_config(SQLITE_DBCONFIG_ENABLE_FKEY, true)
                    .expect("Error while setting SQLITE_DBCONFIG_ENABLE_FKEY");
                conn.execute_batch(
                    &std::fs::read_to_string("init.sql").expect("Couldn't read init.sql"),
                )
                .expect("Couldn't execute init.sql");

                Db { conn }
            }
        }
    }

    pub fn update_bookmark(&self, new: &Bookmark) -> Result<Bookmark> {
        self.conn.execute(
            "UPDATE bookmarks
             SET name = ?1, url = ?2, description = ?3
             WHERE id = ?4",
            params![new.name, new.url, new.description, new.id],
        )?;

        self.conn
            .execute("DELETE FROM tags WHERE bookmark_id = ?", params![new.id])?;

        for tag in &new.tags {
            self.conn
                .execute("INSERT OR IGNORE INTO tags VALUES (?1, ?2)", params![
                    tag.to_lowercase(),
                    new.id
                ])?;
        }

        self.get_bookmark_by_id(new.id)
    }

    pub fn delete_bookmark(&self, ids: &[i64]) -> Result<Vec<Bookmark>> {
        let mut res: Vec<Bookmark> = Vec::new();
        for &id in ids {
            res.push(self.get_bookmark_by_id(id)?);

            self.conn
                .execute("DELETE FROM bookmarks WHERE id = ?", params![id])?;
            self.conn
                .execute("DELETE FROM tags WHERE bookmark_id = ?", params![id])?;
        }

        println!("Deleted: {res:?}");
        Ok(res)
    }

    pub fn set_tag(&self, name: &str, id: i64) -> Result<Bookmark> {
        if let Some(tag_name) = name.strip_prefix('-') {
            self.conn.execute(
                "DELETE FROM tags WHERE tag_name = ?1 AND bookmark_id = ?2",
                params![tag_name, id],
            )?;
        } else {
            self.conn
                .execute("INSERT INTO tags VALUES (?1, ?2)", params![name, id])?;
        }

        self.get_bookmark_by_id(id)
    }

    pub fn delete_tag(&self, name: &str) -> Result<usize> {
        self.conn
            .execute("DELETE FROM tags WHERE tag_name = ?", params![name])
    }

    pub fn rename_tag(&self, old: &str, new: &str) -> Result<usize> {
        self.conn.execute(
            "UPDATE tags SET tag_name = ?1 WHERE tag_name = ?2",
            params![new, old],
        )
    }

    pub fn set_favorite(&self, path: &str) -> Result<i64> {
        self.conn
            .prepare("INSERT INTO favorites VALUES (?)")?
            .insert(params![path])
    }

    pub fn get_favorites(&self) -> Result<Vec<String>> {
        self.conn
            .prepare("SELECT * FROM favorites")?
            .query_map([], |row| row.get("path"))?
            .collect()
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        self.conn
            .prepare(
                "SELECT tag_name, count(bookmark_id) AS bookmarks_count
                 FROM tags WHERE bookmark_id NOT IN
                     (SELECT DISTINCT bookmark_id FROM tags WHERE tag_name = 'private')
                 GROUP BY tag_name",
            )?
            .query_map([], |row| {
                Ok(Tag {
                    tag_name: row.get("tag_name")?,
                    bookmarks_count: row.get("bookmarks_count")?,
                })
            })?
            .collect()
    }

    #[allow(clippy::let_and_return)]
    pub fn get_page(&self, page: &Page) -> Result<Vec<Bookmark>> {
        let offset = page.p.unwrap_or(0);
        let limit = page.limit.unwrap_or(200);
        let sort = page
            .sort
            .clone()
            .unwrap_or("creation_time DESC".to_string());
        let mut stmt = self.conn.prepare(&format!(
            "SELECT * FROM bookmarks LEFT JOIN
                (SELECT group_concat(tag_name) AS tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id
            WHERE id NOT IN
                (SELECT DISTINCT bookmark_id FROM tags WHERE tag_name = 'private')
            ORDER BY {sort}
            LIMIT ?1 OFFSET ?2"
        ))?;
        let res = stmt
            .query_map(params![limit, limit * offset], |row| {
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
            "SELECT * FROM bookmarks LEFT JOIN 
                (SELECT group_concat(tag_name) AS tags, bookmark_id FROM tags WHERE bookmark_id = ?1)
            ON id = bookmark_id WHERE id = ?1",
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
            "SELECT * FROM bookmarks LEFT JOIN
                (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
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

    #[allow(clippy::let_and_return)]
    pub fn get_bookmarks_by_tag(&self, tag_name: &str) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM bookmarks
            JOIN (SELECT group_concat(tag_name) AS tags, bookmark_id FROM tags
                  WHERE bookmark_id IN (SELECT bookmark_id FROM tags WHERE tag_name = ?)
                      AND bookmark_id NOT IN (SELECT bookmark_id FROM tags WHERE tag_name = 'private')
                  GROUP BY bookmark_id)
            ON id = bookmark_id
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
                    tags: row
                        .get("tags")
                        .map(|s: String| s.split(',').map(String::from).collect())?,
                })
            })?
            .collect();

        res
    }

    #[allow(clippy::let_and_return)]
    pub fn get_bookmarks_by_date(&self, date: &str) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM bookmarks LEFT JOIN
                (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id
            WHERE date(creation_time, 'unixepoch', 'localtime') = ?
                AND tags NOT LIKE '%private%'
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
                &format!(
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
        self.conn.query_row_and_then(
            "SELECT count() FROM bookmarks WHERE id NOT IN
                (SELECT bookmark_id FROM tags WHERE tag_name = 'private')",
            [],
            |row| row.get(0),
        )
    }

    fn parse_bookmark(entry: [&str; 2], tags_for_all: &str) -> Bookmark {
        let (url, tags) = entry[1].split_once(' ').map_or(
            (
                entry[1].to_string(),
                tags_for_all.split(' ').map(String::from).collect(),
            ),
            |(url, tags)| {
                (
                    url.to_string(),
                    tags.split(' ')
                        .chain(tags_for_all.split(' '))
                        .map(String::from)
                        .collect(),
                )
            },
        );
        let name = if entry[0].is_empty() {
            url.clone()
        } else {
            entry[0].to_string()
        };

        Bookmark {
            name,
            url,
            tags,
            ..Default::default()
        }
    }

    pub fn insert(&mut self, input: &str, tags_for_all: &str) -> Result<Vec<Bookmark>> {
        let tx = self.conn.transaction()?;
        let mut existing: Vec<String> = Vec::new();
        let mut not_existing: Vec<i64> = Vec::new();

        let bookmarks: Vec<Bookmark> = input
            .lines()
            .array_chunks()
            .map(|entry| Self::parse_bookmark(entry, tags_for_all))
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
                        params![tag_name.to_lowercase(), bookmark_id],
                    )?;
                }

                println!("Inserted: {}", new.url);
                not_existing.push(bookmark_id);
            }
        }

        // println!("Inserted {}", not_existing.len());
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
            "SELECT * FROM bookmarks LEFT JOIN
                (SELECT group_concat(tag_name) as tags, bookmark_id FROM tags GROUP BY bookmark_id)
            ON id = bookmark_id",
        )?;
        let mut data = vec!["title,note,excerpt,url,tags,created,cover,highlights".to_string()];

        data.extend_from_slice(
            &stmt
                .query_map([], |row| {
                    Ok([
                        &row.get("name")?,
                        &format!("\"{}\"", row.get::<&str, String>("description")?),
                        "",
                        &row.get("url")?,
                        &row.get("tags").unwrap_or("".to_string()),
                        &row.get("creation_time")?,
                        "",
                        "",
                    ]
                    .join(","))
                })?
                .collect::<Result<Vec<String>>>()?,
        );

        Ok(data.join("\n"))
    }
}

impl Default for Db {
    fn default() -> Self {
        Self::new("./main.db3")
    }
}
