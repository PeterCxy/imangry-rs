use actix_web::{http, App, AsyncResponder, Form, Responder, HttpRequest, HttpResponse};
use actix_web::error::ErrorInternalServerError;
use app::AngryAppState;
use futures::Future;

pub fn setup_routes(app: App<AngryAppState>) -> App<AngryAppState> {
    app.resource("/p/life", |r| r.method(http::Method::POST).f(life_add))
        .resource("/l/life", |r| r.method(http::Method::GET).f(life_show))
        .resource("/p/life_set", |r| r.method(http::Method::POST).with(life_set))
}

fn life_show(req: &HttpRequest<AngryAppState>) -> impl Responder {
    let state = req.state().clone();
    let db = state.get_db();
    state.spawn_pool(db.get_async_u64("life_secs"))
        .and_then(move |a| {
            Ok(HttpResponse::Ok().body(format!("{}", a)))
        })
        .map_err(|e| {
            // TODO: Properly return the error
            println!("{:?}", e);
            ErrorInternalServerError(e)
        })
        .responder()
}

fn life_add(req: &HttpRequest<AngryAppState>) -> impl Responder {
    // TODO: Rate limit per-IP!
    let state = req.state().clone();
    let db = state.get_db();
    state.spawn_pool(db.get_async_u64("life_secs"))
        .and_then(move |a| {
            let new_a = a + 1;
            state.spawn_pool(db.set_async_u64("life_secs", new_a))
                .and_then(move |_| Ok(HttpResponse::Ok().body(format!("{}", new_a))))
        })
        .map_err(|e| {
            // TODO: Properly return the error
            println!("{:?}", e);
            ErrorInternalServerError(e)
        })
        .responder()
}

#[derive(Deserialize)]
struct SetLifeParams {
    life: u64
}

// ONLY FOR INTERNAL PURPOSES
// THIS SHOULD NOT BE EXPOSED TO THE OUTSIDE OF THE REVERSE PROXY
fn life_set((req, params): (HttpRequest<AngryAppState>, Form<SetLifeParams>)) -> impl Responder {
    let state = req.state().clone();
    let db = state.get_db();
    state.spawn_pool(db.set_async_u64("life_secs", params.life))
        .map(|_| HttpResponse::Ok())
        .map_err(|e| {
            // TODO: Properly return the error
            println!("{:?}", e);
            ErrorInternalServerError(e)
        })
        .responder()
}