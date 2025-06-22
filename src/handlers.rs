use axum::{
    Form, Json,
    extract::{Path, Query, RawQuery, State},
    http::header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    response::{Html, IntoResponse, Redirect},
};
use minijinja::context;

use std::{collections::HashMap, num::ParseIntError};

use crate::{
    AppState,
    types::{Bookmark, MyError, Page},
};

pub async fn add_bookmarks_form(
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Html<String>, MyError> {
    Ok(Html(state.render("index.html",
        context! { bookmarks => state.db.lock()?.insert(
            form.get("urls").ok_or(MyError("no 'urls' field in add_bookmarks_form".to_string()))?,
            form.get("all_tags").ok_or(MyError("no 'all_tags' field in add_bookmarks_form".to_string()))?,
        )?, favorites => state.db.lock()?.get_favorites()? },
    )?))
}

pub async fn update_bookmark_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<Bookmark>,
) -> Result<Html<String>, MyError> {
    Ok(Html(state.render(
        "article.html",
        context! { bookmark => state.db.lock()?.update_bookmark(&Bookmark {
            id,
            url: form.url,
            name: form.name,
            description: form.description,
            tags: form.tags,
            ..Default::default()
        })?},
    )?))
}

pub async fn edit_bookmark(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, MyError> {
    Ok(Html(state.render(
        "edit.html",
        context! { bookmark => state.db.lock()?.get_bookmark_by_id(id)? },
    )?))
}

pub async fn delete_bookmark(
    State(state): State<AppState>,
    RawQuery(rq): RawQuery,
) -> Result<Html<String>, MyError> {
    let parsed_ids: Result<Vec<i64>, ParseIntError> = rq
        .unwrap_or_default()
        .split('&')
        .map(|x| x.split_once('=').unwrap_or_default().1.parse())
        .collect();
    let deleted = state.db.lock()?.delete_bookmark(&parsed_ids?)?;

    Ok(Html(
        deleted
            .iter()
            .map(|bookmark| {
                state
                    .render("article.html", context! { bookmark, deleted => true })
                    .unwrap_or_default()
            })
            .collect::<String>(),
    ))
}

pub async fn set_tag(
    State(state): State<AppState>,
    Path((id, tag)): Path<(i64, String)>,
) -> Result<Html<String>, MyError> {
    Ok(Html(state.render(
        "article.html",
        context! { bookmark => state.db.lock()?.set_tag(&tag, id)? },
    )?))
}

pub async fn tags_page(State(state): State<AppState>) -> Result<Html<String>, MyError> {
    Ok(Html(state.render("tags.html", context! {
        tags => state.db.lock()?.list_tags()?,
        favorites => state.db.lock()?.get_favorites()?
    })?))
}

pub async fn tag_page(
    State(state): State<AppState>,
    Path(tag_name): Path<String>,
) -> Result<Html<String>, MyError> {
    let bookmarks = state.db.lock()?.get_bookmarks_by_tag(&tag_name)?;
    let favorites = state.db.lock()?.get_favorites()?;

    Ok(Html(state.render(
        "index.html",
        context! { bookmarks, favorites, tag_name },
    )?))
}

pub async fn rename_tag(
    State(state): State<AppState>,
    Path(old): Path<String>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Redirect, MyError> {
    state.db.lock()?.rename_tag(
        &old,
        form.get("new")
            .ok_or(MyError("no 'new' field in form".to_string()))?,
    )?;
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
    state.db.lock()?.set_favorite(&format!("/tags/{name}"))?;
    Ok(([("HX-Refresh", "true")], ""))
}

pub async fn index(State(state): State<AppState>) -> Result<Html<String>, MyError> {
    Ok(Html(state.render("index.html", context! {
        bookmarks => state.db.lock()?.get_bookmarks_by_tag("imp")?,
        favorites => state.db.lock()?.get_favorites()?
    })?))
}

pub async fn page(
    State(state): State<AppState>,
    Query(page): Query<Page>,
) -> Result<Html<String>, MyError> {
    let db = state.db.lock()?;
    let number = db.count_all().unwrap_or_default();

    Ok(Html(state.render("index.html", context! {
        bookmarks => db.get_page(&page)?,
        number,
        page => page.p.unwrap_or_default(),
        pages => number.div_ceil(page.limit.unwrap_or(200)),
        favorites => db.get_favorites()?
    })?))
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
            .into_iter()
            .map(|tag| tag.tag_name)
            .collect(),
    ))
}
