use actix_web::{
    error::ErrorInternalServerError,
    get, post,
    web::{Form, Path, Query, Redirect},
    HttpResponse, Responder, Result,
};
use futures_util::StreamExt as _;

use std::collections::BTreeSet;

use crate::database;
use crate::templates;
use crate::types::{AddUrlsForm, Bookmark, EditBookmarkForm, Page, Search};

#[post("/add-urls")]
async fn add_urls_form(Form(form): Form<AddUrlsForm>) -> Result<impl Responder> {
    database::insert_from_lines(form.urls).map_err(ErrorInternalServerError)?;
    Ok(Redirect::to("/all").see_other())
}

#[post("/")]
async fn add_file_form(mut payload: actix_multipart::Multipart) -> Result<impl Responder> {
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let mut res = "".to_string();

        while let Some(chunk) = field.next().await {
            res.push_str(std::str::from_utf8(&chunk?)?);
        }

        database::insert_from_lines(res).map_err(ErrorInternalServerError)?;
    }

    Ok(Redirect::to("/all").see_other())
}

#[post("/update-bookmark/{id:\\d+}")]
async fn update_bookmark_form(
    id: Path<u64>,
    form: Form<EditBookmarkForm>,
) -> Result<impl Responder> {
    database::update_bookmark(Bookmark {
        id: id.into_inner(),
        name: form.name.clone(),
        url: form.url.clone(),
        creation_time: 0,
        tags: form
            .tags
            .split(' ')
            .map(String::from)
            .collect::<BTreeSet<_>>(),
    })
    .map_err(ErrorInternalServerError)?;
    Ok(Redirect::to("/all").see_other())
}

#[get("/search")]
async fn search(q: Query<Search>) -> Result<HttpResponse> {
    templates::index_page(database::search(q.q.clone()).map_err(ErrorInternalServerError)?)
        .map_or_else(
            |err| Err(ErrorInternalServerError(err)),
            |body| Ok(HttpResponse::Ok().body(body)),
        )
}

#[get("edit-bookmark/{id:\\d+}/")]
async fn edit_page(id: Path<u64>) -> Result<HttpResponse> {
    templates::edit_page(
        database::get_bookmark_by_id(id.into_inner()).map_err(ErrorInternalServerError)?,
    )
    .map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("delete-bookmark/{id:\\d+}/")]
async fn delete_bookmark(id: Path<u64>) -> Result<impl Responder> {
    database::delete_bookmark(id.into_inner()).map_err(ErrorInternalServerError)?;
    Ok(Redirect::to("/all").see_other())
}

#[get("tags/")]
async fn tags_page() -> Result<HttpResponse> {
    templates::tags_page(database::list_tags().map_err(ErrorInternalServerError)?).map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("tags/{name}/")]
async fn tag_page(name: Path<String>) -> Result<HttpResponse> {
    templates::index_page(
        database::get_bookmarks_by_tag(name.into_inner()).map_err(ErrorInternalServerError)?,
    )
    .map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("date/{date}/")]
async fn date_page(date: Path<String>) -> Result<HttpResponse> {
    templates::index_page(
        database::get_bookmarks_by_date(date.into_inner()).map_err(ErrorInternalServerError)?,
    )
    .map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}

#[get("/all")]
async fn page(page: Query<Page>) -> Result<HttpResponse> {
    templates::index_page(
        database::list_all(page.p.unwrap_or_default()).map_err(ErrorInternalServerError)?,
    )
    .map_or_else(
        |err| Err(ErrorInternalServerError(err)),
        |body| Ok(HttpResponse::Ok().body(body)),
    )
}
