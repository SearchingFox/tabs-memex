CREATE TABLE IF NOT EXISTS bookmarks (
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
END;
