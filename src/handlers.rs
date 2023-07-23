use axum::{
    extract::{Path, Query, State},
    http::header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    response::{Html, IntoResponse, Redirect, Response, Result},
    Form, Json,
};
use sailfish::TemplateOnce;

use std::collections::HashMap;

use crate::{
    types::{Bookmark, EditPage, IndexPage, MyError, Page, Tag, Tags},
    AppState,
};

pub async fn add_bookmarks_form(
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, MyError> {
    let db = &mut state.db.lock()?;
    let duplicates = db.insert_from_lines(form.get("urls").ok_or(MyError::InternalError(
        "no 'urls' field in form".to_string(),
    ))?)?;
    let len = duplicates.len();

    if len == 0 {
        return Ok(Redirect::to("/all").into_response());
    }

    Ok(Html(
        IndexPage {
            bookmarks: duplicates,
            number: len,
            page: 0,
            pages: 0,
        }
        .render_once()?,
    )
    .into_response())
}

pub async fn update_bookmark_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<Bookmark>,
) -> Result<Html<String>, MyError> {
    let db = &state.db.lock()?;
    db.update_bookmark(Bookmark {
        id,
        url: form.url,
        name: form.name,
        description: form.description,
        tags: form.tags,
        ..Default::default()
    })?;

    Ok(Html(db.get_bookmark_by_id(id)?.render_once()?))
}

pub async fn edit_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, MyError> {
    let bm = state.db.lock()?.get_bookmark_by_id(id)?;
    let ep = EditPage {
        id: bm.id,
        name: bm.name,
        url: bm.url,
        description: bm.description,
        tags: bm.tags,
    };
    Ok(Html(ep.render_once()?))
}

pub async fn delete_bookmark(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, MyError> {
    let db = &state.db.lock()?;

    Ok(Html(format!(
        "<div style=\"color: red\">{:?} removed</div>",
        db.delete_bookmark(id)?
    )))
}

pub async fn set_tag(
    State(state): State<AppState>,
    Path((id, tag)): Path<(i64, String)>,
) -> Result<Html<String>, MyError> {
    let db = &state.db.lock()?;

    Ok(Html(db.set_tag(&tag, id)?.render_once()?))
}

pub async fn tags_page(State(state): State<AppState>) -> Result<Html<String>, MyError> {
    let db = &state.db.lock()?;
    Ok(Html(
        Tags {
            tags: db.list_tags()?,
        }
        .render_once()?,
    ))
}

pub async fn tag_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Html<String>, MyError> {
    let db = &state.db.lock()?;
    let found = db.get_bookmarks_by_tag(&name)?;
    let len = found.len();

    Ok(Html(
        IndexPage {
            bookmarks: found,
            number: len,
            page: 0,
            pages: 0,
        }
        .render_once()?,
    ))
}

pub async fn rename_tag(
    State(state): State<AppState>,
    Path(old): Path<String>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Redirect, MyError> {
    let db = &state.db.lock()?;
    db.rename_tag(
        &old,
        form.get("new")
            .ok_or(MyError::InternalError("no 'new' field in form".to_string()))?,
    )?;

    Ok(Redirect::to("/tags"))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Redirect, MyError> {
    let db = &state.db.lock()?;
    db.delete_tag(&name)?;

    Ok(Redirect::to("/tags"))
}

pub async fn index(State(state): State<AppState>) -> Result<Html<String>, MyError> {
    let found = state.db.lock()?.get_bookmarks_by_tag("imp")?;
    let len = found.len();

    Ok(Html(
        IndexPage {
            bookmarks: found,
            number: len,
            page: 0,
            pages: 0,
        }
        .render_once()?,
    ))
}

pub async fn page(
    State(state): State<AppState>,
    Query(page): Query<HashMap<String, usize>>,
) -> Result<Html<String>, MyError> {
    let offset = page.get("p").cloned().unwrap_or_default();
    let limit = page.get("limit").cloned().unwrap_or(200);
    let db = &state.db.lock()?;
    let number = db.count_all().unwrap_or_default();

    Ok(Html(
        IndexPage {
            bookmarks: db.get_page(Page { offset, limit })?,
            number,
            page: offset,
            pages: number / limit + 1,
        }
        .render_once()?,
    ))
}

pub async fn search(
    State(state): State<AppState>,
    q: Query<HashMap<String, String>>,
) -> Result<Html<String>, MyError> {
    let db = &state.db.lock()?;
    let found = match q.iter().next() {
        Some((k, v)) if k == "d" => db.get_bookmarks_by_date(v)?,
        Some((k, v)) if k == "q" => db.search(v)?,
        _ => Vec::new(),
    };
    let len = found.len();

    Ok(Html(
        IndexPage {
            bookmarks: found,
            number: len,
            page: 0,
            pages: 0,
        }
        .render_once()?,
    ))
}

pub async fn export_csv(State(state): State<AppState>) -> Result<Response, MyError> {
    let headers = [
        (CONTENT_TYPE, "text/csv; charset=utf-8"),
        (CONTENT_DISPOSITION, "attachment; filename=\"export.csv\""),
    ];
    let body = state.db.lock()?.export_csv()?;

    Ok((headers, body).into_response())
}

pub async fn all_tags(State(state): State<AppState>) -> Result<Json<Vec<String>>, MyError> {
    let body = state
        .db
        .lock()?
        .list_tags()?
        .into_iter()
        .map(
            |Tag {
                 tag_name,
                 bookmarks_count: _,
             }| tag_name,
        )
        .collect();

    Ok(Json(body))
}
