extern crate actix;
extern crate actix_web;
extern crate bytes;
extern crate byteorder;
extern crate rand;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate sled;
extern crate tokio_threadpool;

mod util;
mod db_sync;
#[macro_use]
mod app;
mod life;
mod url;
mod paste;

use actix_web::{http, server, App, Path, Responder};
use sled::{ConfigBuilder, Tree};

fn main() {
    let actix_sys = actix::System::new("imangry");
    let db = open_db("data/db");
    let db_sync_addr = db_sync::DbSyncActor::start_actor(&db);
    let state = app::AngryAppState::new(db, db_sync_addr);
    server::new(move || {
        let mut app = App::with_state(state.clone())
            .route("/test_index.html", http::Method::GET, test_index);
        app = life::setup_routes(app);
        app = url::setup_routes(app);
        app = paste::setup_routes(app);
        app
    }).bind("127.0.0.1:60324").unwrap().start();
    actix_sys.run();
}

fn test_index(_info: Path<()>) -> impl Responder {
    "I'm angry!"
}

fn open_db(db_path: &str) -> Tree {
    let config = ConfigBuilder::new()
        .path(db_path.to_owned())
        .io_buf_size(65535)
        .min_items_per_segment(4) // For maximum 16k key/value pair
        .build();
    Tree::start(config).unwrap()
}