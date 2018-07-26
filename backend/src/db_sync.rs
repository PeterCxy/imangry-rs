use actix::prelude::*;
use sled::Tree;
use std::time::{Duration, Instant};

const MIN_SYNC_INTERVAL: u64 = 2000;

// Actor to flush the Database on command
// We use an actor to perform the work
// So that we can control how often should it be flushed
// Also, we can delegate the work complete to background
// Because this should be used as an sync actor.
pub struct DbSyncActor {
    db: Tree,
    last_sync: Instant
}

impl DbSyncActor {
    pub fn start_actor(db: &Tree) -> Addr<DbSyncActor> {
        let db_clone = db.clone();
        SyncArbiter::start(1, move || Self::new(db_clone.clone()))
    }

    fn new(db: Tree) -> DbSyncActor {
        DbSyncActor {
            db,
            last_sync: Instant::now()
        }
    }
}

impl Actor for DbSyncActor {
    type Context = SyncContext<Self>;
}

pub struct DbSyncCommand;

impl Message for DbSyncCommand {
    type Result = ();
}

impl Handler<DbSyncCommand> for DbSyncActor {
    type Result = ();

    fn handle(&mut self, _msg: DbSyncCommand, _ctx: &mut Self::Context) -> Self::Result {
        let now = Instant::now();
        // Only flush every MIN_SYNC_INTERVAL
        if now.duration_since(self.last_sync) <= Duration::from_millis(MIN_SYNC_INTERVAL) {
            return;
        }
        // Just ignore any error that may occur on flush
        self.db.flush().map_err(|e| println!("{:?}", e)).unwrap_or(());
        self.last_sync = Instant::now();
        return;
    }
}