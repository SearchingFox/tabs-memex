use tera::{Context, Result, Tera};

use crate::types::{Bookmark, Tag};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        }
    };
}

pub fn index_page(bms: Vec<Bookmark>, num: u64, pages: u64) -> Result<String> {
    let mut ctx = Context::new();
    ctx.insert("bookmarks", &bms);
    ctx.insert("number", &num);
    ctx.insert("pages", &pages);
    TEMPLATES.render("index.html", &ctx)
}

pub fn tags_page(tags: Vec<Tag>) -> Result<String> {
    let mut ctx = Context::new();
    ctx.insert("tags", &tags);
    TEMPLATES.render("tags.html", &ctx)
}

pub fn edit_page(bookmark: Bookmark) -> Result<String> {
    TEMPLATES.render("edit.html", &Context::from_serialize(bookmark)?)
}
