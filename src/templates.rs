use std::collections::HashMap;

use serde_json::value::{to_value, Value};
use tera::{Context, Result, Tera};

use crate::types::{Bookmark, Tag};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec!["html", ".sql"]);
        tera.register_filter("do_nothing", do_nothing_filter);
        tera
    };
}

fn do_nothing_filter(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("do_nothing_filter", "value", String, value);
    Ok(to_value(s).unwrap())
}

pub fn index_page(bms: Vec<Bookmark>) -> Result<String> {
    let mut ctx = Context::new();
    ctx.insert("bookmarks", &bms);
    TEMPLATES.render("index.html", &ctx)
}

pub fn tags_page(tags: Vec<Tag>) -> Result<String> {
    let mut ctx = Context::new();
    ctx.insert("tags", &tags);
    TEMPLATES.render("tags.html", &ctx)
}

pub fn edit_page(bookmark: Bookmark) -> Result<String> {
    let mut ctx = Context::new();
    ctx.insert("bookmark", &bookmark);
    TEMPLATES.render("edit.html", &ctx)
}
