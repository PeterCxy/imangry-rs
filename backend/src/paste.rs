use actix_web::{http, App, AsyncResponder, Responder, HttpMessage, HttpRequest, HttpResponse};
use actix_web::error::{ErrorNotFound, ErrorInternalServerError};
use app::{AngryError, AngryAppState};
use bytes::Bytes;
use futures::Future;
use futures::future::{self, Either};
use htmlescape;
use util;

pub fn setup_routes(app: App<AngryAppState>) -> App<AngryAppState> {
    app.resource("/p/text", |r| r.method(http::Method::POST).f(paste))
        .resource("/t/{id}", |r| r.method(http::Method::GET).f(show_paste))
}

fn paste(req: &HttpRequest<AngryAppState>) -> impl Responder {
    // TODO: Maybe we can make an abstraction of the common logic
    // in this module and the url module?
    let base_url = util::conn_scheme_host_port(&req.request().connection_info());
    let state = req.state().clone();
    let state_1 = state.clone();
    let db = state.get_db();
    let db_1 = db.clone();

    req.body().limit(15 * 1024)
        .map_err(|e| e.into())
        .and_then(move |body: Bytes| {
            let id = util::rand_str(6);
            state.spawn_pool(db.get_async_utf8(format!("paste_{}", id)))
                .map(|r| (r, id, body))
        })
        .and_then(move |(r, id, body)| {
            if r != "" {
                // TODO: Retry if we hit an existing one
                println!("{}", r);
                Either::A(future::err(AngryError::String("WTF".into())))
            } else {
                Either::B(
                    state_1.spawn_pool(db_1.set_async(format!("paste_{}", id), body.to_vec()))
                        .map(|_| id)
                )
            }
        })
        .map(move |id| {
            format!("{}/t/{}", base_url, id)
        })
        .map_err(|e| {
            // TODO: Properly return the error
            // Maybe we need a class to convert AngryError into corresponding HTTP errors?
            // e.g. maybe return 404 if we cannot parse the request string?
            println!("{:?}", e);
            ErrorInternalServerError(e)
        })
        .responder()
}

fn show_paste(req: &HttpRequest<AngryAppState>) -> impl Responder {
    let ua = req.headers().get(http::header::USER_AGENT)
        .and_then(|r| {
            r.to_str().ok()
        }).unwrap_or("").to_owned();
    let id = req.match_info().get("id").unwrap().to_owned();
    let state = req.state().clone();
    let db = state.get_db();
    state.spawn_pool(db.get_async_utf8(format!("paste_{}", id)))
        .map_err(|e| {
            println!("{:?}", e);
            ErrorInternalServerError(e)
        })
        .and_then(move |p| {
            if p != "" {
                let mut resp = HttpResponse::Ok();
                if util::ua_is_browser(&ua) {
                    // Show a rendered HTML for GUI browsers
                    resp.header(http::header::CONTENT_TYPE, "text/html");
                    Ok(resp.body(
                        include_str!("../../template/code.html")
                            .replace("{{code}}", &htmlescape::encode_minimal(&p))))
                } else {
                    resp.header(http::header::CONTENT_TYPE, "text/plain");
                    Ok(resp.body(p))
                }
            } else {
                Err(ErrorNotFound("Paste data not recorded"))
            }
        })
        .responder()
}