extern crate actix_web;
extern crate byteorder;
extern crate futures;
extern crate sled;
extern crate tokio_threadpool;

#[macro_use]
mod app;
mod life;

use actix_web::{http, server, App, Path, Responder};

fn main() {
    let state = app::AngryAppState::new("data".to_owned());
    server::new(move || {
        let mut app = App::with_state(state.clone())
            .route("/test_index.html", http::Method::GET, test_index);
        app = life::setup_routes(app);
        app
    })
    .bind("127.0.0.1:60324").unwrap()
    .run()
}

fn test_index(_info: Path<()>) -> impl Responder {
    "I'm angry!"
}