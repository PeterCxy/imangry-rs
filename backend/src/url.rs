use actix_web::{http, App, AsyncResponder, Responder, HttpMessage, HttpRequest, HttpResponse};
use actix_web::error::{ErrorNotFound, ErrorInternalServerError};
use app::{AngryError, AngryAppState};
use futures::Future;
use futures::future::{self, Either};
use util;

pub fn setup_routes(app: App<AngryAppState>) -> App<AngryAppState> {
    app.resource("/p/url", |r| r.method(http::Method::POST).f(shorten_url))
        .resource("/u/{short}", |r| r.method(http::Method::GET).f(show_shortened_url))
}

#[derive(Deserialize)]
struct ShortenUrlData {
    u: String
}

fn shorten_url(req: &HttpRequest<AngryAppState>) -> impl Responder {
    let base_url = util::conn_scheme_host_port(&req.request().connection_info());
    let state = req.state().clone();
    let state_1 = state.clone();
    let db = state.get_db();
    let db_1 = db.clone();

    req.urlencoded::<ShortenUrlData>()
        .map_err(|e| e.into())
        .and_then(move |url| {
            // TODO: Validate the URL!!!!
            let short = util::rand_str(6);
            state.spawn_pool(db.get_async_utf8(format!("url_{}", short)))
                .map(|r| (r, short, url))
        })
        .and_then(move |(r, short, url)| {
            if r != "" {
                // TODO: Retry if we hit an existing one
                println!("{}", r);
                Either::A(future::err(AngryError::String("WTF".into())))
            } else {
                Either::B(
                    state_1.spawn_pool(db_1.set_async(format!("url_{}", short), url.u))
                        .map(|_| short)
                )
            }
        })
        .map(move |short| {
            format!("{}/u/{}", base_url, short)
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

fn show_shortened_url(req: &HttpRequest<AngryAppState>) -> impl Responder {
    let short = req.match_info().get("short").unwrap().to_owned();
    let state = req.state().clone();
    let db = state.get_db();
    state.spawn_pool(db.get_async_utf8(format!("url_{}", short)))
        .map_err(|e| {
            println!("{:?}", e);
            ErrorInternalServerError(e)
        })
        .and_then(|u| {
            if u != "" {
                Ok(HttpResponse::Found().header("Location", u).finish())
            } else {
                Err(ErrorNotFound("Url not recorded"))
            }
        })
        .responder()
}