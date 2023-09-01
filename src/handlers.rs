use axum::{
    extract::{Path, Query, State},
    http::header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    response::{Html, IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::Form;
use minijinja::context;

use std::collections::HashMap;

use crate::{
    types::{Bookmark, MyError, Page},
    AppState,
};

pub async fn add_bookmarks_form(
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Html<String>, MyError> {
    Ok(Html(state.env.get_template("index.html")?.render(
        context! { bookmarks => state.db.lock()?.insert(
            form.get("urls").ok_or("no 'urls' field in add_bookmarks_form")?,
            form.get("all_tags").ok_or("no 'all_tags' field in add_bookmarks_form")?,
        )? },
    )?))
}

pub async fn update_bookmark_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<Bookmark>,
) -> Result<Html<String>, MyError> {
    Ok(Html(state.env.get_template("article.html")?.render(
        context! { bookmark => state.db.lock()?.update_bookmark(Bookmark {
            id,
            url: form.url,
            name: form.name,
            description: form.description,
            tags: form.tags,
            ..Default::default()
        })?},
    )?))
}

pub async fn edit_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, MyError> {
    Ok(Html(state.env.get_template("edit.html")?.render(
        context! { bookmark => state.db.lock()?.get_bookmark_by_id(id)? },
    )?))
}

pub async fn delete_bookmark(
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, Vec<i64>>>,
) -> Result<Html<String>, MyError> {
    let res: Vec<Bookmark> = state.db.lock()?.delete_bookmark(form.get("ids").unwrap())?;
    Ok(Html(
        res.iter()
            .map(|bookmark| {
                state
                    .render("article.html", context! { bookmark, deleted => true })
                    .unwrap_or("".to_string())
            })
            .collect::<String>(),
    ))
}

pub async fn set_tag(
    State(state): State<AppState>,
    Path((id, tag)): Path<(i64, String)>,
) -> Result<Html<String>, MyError> {
    let bookmark = state.db.lock()?.set_tag(&tag, id)?;
    Ok(Html(state.render("article.html", context! { bookmark })?))
}

pub async fn tags_page(State(state): State<AppState>) -> Result<Html<String>, MyError> {
    let tags = state.db.lock()?.list_tags()?;
    Ok(Html(state.render("tags.html", context! { tags })?))
}

pub async fn tag_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Html<String>, MyError> {
    let bookmarks = state.db.lock()?.get_bookmarks_by_tag(&name)?;
    let favorites = state.db.lock()?.get_favorites()?;
    Ok(Html(
        state.render("index.html", context! { bookmarks, favorites })?,
    ))
}

pub async fn rename_tag(
    State(state): State<AppState>,
    Path(old): Path<String>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Redirect, MyError> {
    state
        .db
        .lock()?
        .rename_tag(&old, form.get("new").ok_or("no 'new' field in form")?)?;
    Ok(Redirect::to("/tags"))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, MyError> {
    state.db.lock()?.delete_tag(&name)?;
    Ok(([("HX-Refresh", "true")], ""))
}

pub async fn set_favorite(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, MyError> {
    state.db.lock()?.set_favorite(&name)?;
    Ok(([("HX-Refresh", "true")], ""))
}

pub async fn index(State(state): State<AppState>) -> Result<Html<String>, MyError> {
    Ok(Html(state.render(
        "index.html",
        context! {
            bookmarks => state.db.lock()?.get_bookmarks_by_tag("imp")?,
            favorites => state.db.lock()?.get_favorites()?
        },
    )?))
}

pub async fn page(
    State(state): State<AppState>,
    Query(page): Query<HashMap<String, usize>>,
) -> Result<Html<String>, MyError> {
    let offset = page.get("p").cloned().unwrap_or_default();
    let limit = page.get("limit").cloned().unwrap_or(200);
    let db = state.db.lock()?;
    let number = db.count_all().unwrap_or_default();
    let favorites = db.get_favorites()?;

    Ok(Html(state.render(
        "index.html",
        context! {
            bookmarks => db.get_page(Page { offset, limit })?,
            number,
            page => offset,
            pages => number.div_ceil(limit),
            favorites
        },
    )?))
}

pub async fn search(
    State(state): State<AppState>,
    q: Query<HashMap<String, String>>,
) -> Result<Html<String>, MyError> {
    let db = state.db.lock()?;
    let bookmarks = match q.iter().next() {
        Some((k, v)) if k == "d" => db.get_bookmarks_by_date(v)?,
        Some((k, v)) if k == "q" => db.search(v)?,
        _ => Vec::new(),
    };
    let favorites = db.get_favorites()?;

    Ok(Html(
        state.render("index.html", context! { bookmarks, favorites })?,
    ))
}

pub async fn export_csv(State(state): State<AppState>) -> Result<impl IntoResponse, MyError> {
    Ok((
        [
            (CONTENT_TYPE, "text/csv; charset=utf-8"),
            (CONTENT_DISPOSITION, "attachment; filename=\"export.csv\""),
        ],
        state.db.lock()?.export_csv()?,
    ))
}

pub async fn all_tags(State(app_state): State<AppState>) -> Result<Json<Vec<String>>, MyError> {
    Ok(Json(
        app_state
            .db
            .lock()?
            .list_tags()?
            .iter()
            .map(|tag| tag.tag_name.clone())
            .collect(),
    ))
}
