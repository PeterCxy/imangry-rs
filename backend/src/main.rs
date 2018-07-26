extern crate actix;
extern crate actix_web;
extern crate byteorder;
extern crate futures;
extern crate sled;
extern crate tokio_threadpool;

mod db_sync;
#[macro_use]
mod app;
mod life;

use actix_web::{http, server, App, Path, Responder};
use sled::{ConfigBuilder, Tree};

fn main() {
    let actix_sys = actix::System::new("imangry");
    let db = open_db("data");
    let db_sync_addr = db_sync::DbSyncActor::start_actor(&db);
    let state = app::AngryAppState::new(db, db_sync_addr);
    server::new(move || {
        let mut app = App::with_state(state.clone())
            .route("/test_index.html", http::Method::GET, test_index);
        app = life::setup_routes(app);
        app
    })
    .bind("127.0.0.1:60324").unwrap()
    .start();
    actix_sys.run();
}

fn test_index(_info: Path<()>) -> impl Responder {
    "I'm angry!"
}

fn open_db(db_path: &str) -> Tree {
    let config = ConfigBuilder::new()
        .path(db_path.to_owned())
        .build();
    Tree::start(config).unwrap()
}