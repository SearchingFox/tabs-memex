use actix_files::NamedFile;
use actix_web::{
    error::ErrorInternalServerError,
    get,
    http::header::REFERER,
    post,
    web::{Form, Path, Query, Redirect},
    HttpRequest, HttpResponse, Responder, Result,
};
use futures_util::StreamExt as _;
use sailfish::TemplateOnce;

use std::collections::HashMap;

use crate::database;
use crate::types::{Bookmark, Index, Tags};

fn get_referer(req: HttpRequest) -> Result<String> {
    req.headers()
        .get(REFERER)
        .map_or(Ok("/all"), |x| x.to_str())
        .map(String::from)
        .map_err(ErrorInternalServerError)
}

#[post("/all")]
async fn add_file_form(
    req: HttpRequest,
    mut payload: actix_multipart::Multipart,
) -> Result<impl Responder> {
    let mut rs: Vec<Bookmark> = Vec::new();
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let mut res = "".to_string();

        while let Some(chunk) = field.next().await {
            res.push_str(std::str::from_utf8(&chunk?)?);
        }

        rs.append(&mut database::insert_from_lines(res));
        // .map_err(ErrorInternalServerError)?
    }

    if rs.is_empty() {
        Ok(HttpResponse::SeeOther()
            .append_header(("Location", get_referer(req)?))
            .finish())
    } else {
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(
                Index {
                    bookmarks: rs.clone(),
                    number: rs.len(),
                    pg: 0,
                    pages: 0,
                }
                .render_once()
                .map_err(ErrorInternalServerError)?,
            ))
    }
}

#[post("/add-urls")]
async fn add_urls_form(
    req: HttpRequest,
    Form(form): Form<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let res = database::insert_from_lines(form.get("urls").cloned().unwrap());
    if res.is_empty() {
        Ok(HttpResponse::SeeOther()
            .append_header(("Location", get_referer(req)?))
            .finish())
    } else {
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(
                Index {
                    bookmarks: res.clone(),
                    number: res.len(),
                    pg: 0,
                    pages: 0,
                }
                .render_once()
                .map_err(ErrorInternalServerError)?,
            ))
    }
}

#[post("/edit-bookmark/{id:\\d+}")]
async fn update_bookmark_form(id: Path<i64>, form: Form<Bookmark>) -> Result<impl Responder> {
    database::update_bookmark(Bookmark {
        id: id.into_inner(),
        name: form.name.clone(),
        url: form.url.clone(),
        creation_time: 0,
        description: form.description.clone(),
        tags: form.tags.clone(),
    })
    .map_err(ErrorInternalServerError)?;

    Ok(Redirect::to("/all").see_other())
}

#[get("/edit-bookmark/{id:\\d+}")]
async fn edit_page(req: HttpRequest, id: Path<u64>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            database::get_bookmark_by_id(id.into_inner())
                .map_err(ErrorInternalServerError)?
                .render_once()
                .map_err(ErrorInternalServerError)?,
        ))
}

#[get("/delete-bookmark/{id:\\d+}")]
async fn delete_bookmark(req: HttpRequest, id: Path<u64>) -> Result<impl Responder> {
    database::delete_bookmark(id.into_inner()).map_err(ErrorInternalServerError)?;

    Ok(Redirect::to(get_referer(req)?).see_other())
}

#[get("/set-tag/{id}/{tag}")]
async fn set_tag(req: HttpRequest, path: Path<(i64, String)>) -> Result<impl Responder> {
    let (id, tag) = path.into_inner();
    database::set_tag(id, tag).map_err(ErrorInternalServerError)?;

    Ok(Redirect::to(get_referer(req)?).see_other())
}

#[get("/tags")]
async fn tags_page() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            Tags {
                tags: database::list_tags().map_err(ErrorInternalServerError)?,
            }
            .render_once()
            .map_err(ErrorInternalServerError)?,
        ))
}

#[get("/tags/{name}")]
async fn tag_page(name: Path<String>) -> Result<HttpResponse> {
    let found =
        database::get_bookmarks_by_tag(name.into_inner()).map_err(ErrorInternalServerError)?;
    let len = found.len();

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            Index {
                bookmarks: found,
                number: len,
                pg: 0,
                pages: 0,
            }
            .render_once()
            .map_err(ErrorInternalServerError)?,
        ))
}

#[post("/rename-tag/{old}")]
async fn rename_tag(
    req: HttpRequest,
    old: Path<String>,
    form: Form<HashMap<String, String>>,
) -> Result<impl Responder> {
    database::rename_tag(old.into_inner(), form.get("new").unwrap())
        .map_err(ErrorInternalServerError)?;

    Ok(Redirect::to(get_referer(req)?).see_other())
}

#[get("/delete-tag/{name}")]
async fn delete_tag(req: HttpRequest, name: Path<String>) -> Result<impl Responder> {
    database::delete_tag(name.into_inner()).map_err(ErrorInternalServerError)?;

    Ok(Redirect::to(get_referer(req)?).see_other())
}

#[get("/")]
async fn index() -> Result<HttpResponse> {
    let found =
        database::get_bookmarks_by_tag("imp".to_string()).map_err(ErrorInternalServerError)?;
    let len = found.len();

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            Index {
                bookmarks: found,
                number: len,
                pg: 0,
                pages: 0,
            }
            .render_once()
            .map_err(ErrorInternalServerError)?,
        ))
}

#[get("/all")]
async fn page(page: Query<HashMap<String, usize>>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            Index {
                bookmarks: database::list_all(page.get("p").cloned().unwrap_or_default())
                    .map_err(ErrorInternalServerError)?,
                number: database::count_all().unwrap_or(0),
                pg: page.get("p").cloned().unwrap_or_default(),
                pages: database::count_all().unwrap_or(0) / 100 + 1,
            }
            .render_once()
            .map_err(ErrorInternalServerError)?,
        ))
}

#[get("/search")]
async fn search(q: Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let found = match q.iter().next() {
        Some((k, v)) if k == "d" => {
            database::get_bookmarks_by_date(v).map_err(ErrorInternalServerError)?
        }
        Some((k, v)) if k == "q" => database::search(v).map_err(ErrorInternalServerError)?,
        _ => Vec::new(),
    };
    let len = found.len();

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            Index {
                bookmarks: found,
                number: len,
                pg: 0,
                pages: 0,
            }
            .render_once()
            .map_err(ErrorInternalServerError)?,
        ))
}

#[get("/style.css")]
async fn style() -> impl Responder {
    NamedFile::open_async("./templates/style.css").await
}
