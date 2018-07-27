use actix::Addr;
use actix_web::error::UrlencodedError;
use byteorder::{ByteOrder, LittleEndian};
use db_sync::{DbSyncActor, DbSyncCommand};
use futures::{Canceled, Future};
use futures::future::poll_fn;
use futures::sync::oneshot::channel;
use sled::{Tree, Error as DbError};
use std::str;
use std::sync::Arc;
use tokio_threadpool::{BlockingError, blocking, ThreadPool};

#[derive(Debug)]
#[allow(dead_code)]
pub enum AngryError<E> {
    BlockingError(BlockingError),
    DbError(DbError<E>),
    Utf8Error(str::Utf8Error),
    UrlencodedError(UrlencodedError),
    String(String),
    Plain(E),
    Nothing
}

impl<E> ::std::fmt::Display for AngryError<E> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "TODO")
    }
}

impl<E> From<BlockingError> for AngryError<E> {
    fn from(err: BlockingError) -> AngryError<E> {
        AngryError::BlockingError(err)
    }
}

impl<E> From<DbError<E>> for AngryError<E> {
    fn from(err: DbError<E>) -> AngryError<E> {
        AngryError::DbError(err)
    }
}

impl<E> From<Canceled> for AngryError<E> {
    fn from(_err: Canceled) -> AngryError<E> {
        AngryError::Nothing
    }
}

impl<E> From<str::Utf8Error> for AngryError<E> {
    fn from(err: str::Utf8Error) -> AngryError<E> {
        AngryError::Utf8Error(err)
    }
}

impl<E> From<UrlencodedError> for AngryError<E> {
    fn from(err: UrlencodedError) -> AngryError<E> {
        AngryError::UrlencodedError(err)
    }
}

macro_rules! flatten_error {
    ($x:expr) => {
        $x.then(|r| {
            match r {
                Err(err) => Err(err.into()),
                Ok(res) => match res {
                    Err(err) => Err(err.into()),
                    Ok(res) => Ok(res)
                }
            }
        })
    }
}

// Shared state for this application
// Mostly, the database and the thread pool
#[derive(Clone)]
pub struct AngryAppState {
    db: Tree,
    db_sync_actor: Addr<DbSyncActor>,
    pool: Arc<ThreadPool>
}

impl AngryAppState {
    pub fn new(db: Tree, db_sync_actor: Addr<DbSyncActor>) -> AngryAppState {
        AngryAppState {
            db,
            db_sync_actor,
            pool: Arc::new(ThreadPool::new())
        }
    }

    pub fn get_db(&self) -> DbExt {
        DbExt {
            db: self.db.clone(),
            db_sync_actor: self.db_sync_actor.clone()
        }
    }

    // Execute a Future on the thread pool context created in this object
    // and return its results as an identical future
    // This is used for futures that contain `blocking()` invocations
    // and actix-web itself does not run on tokio's threadpool infrastructure
    pub fn spawn_pool<F, I, E>(&self, f: F) -> impl Future<Item = I, Error = AngryError<E>>
        where F: Future<Item = I, Error = AngryError<E>> + Send + 'static,
              I: Send + 'static,
              E: Send + 'static {
        let (tx, rx) = channel();
        self.pool.spawn(
            f.then(move |r| tx.send(r).map_err(|_| ())));
        flatten_error!(rx)
    }
}

#[derive(Clone)]
pub struct DbExt {
    db: Tree,
    db_sync_actor: Addr<DbSyncActor>
}

// Convenience methods for operation on the sled database
// All of these methods should be run in a ThreadPool context
impl DbExt {
    pub fn get_async<K: Into<Vec<u8>>>(&self, key: K) -> impl Future<Item = Option<Vec<u8>>, Error = AngryError<()>> {
        let db = self.db.clone();
        let k = key.into();
        flatten_error!(poll_fn(move || blocking(|| {
            let res = db.get(&k);
            //db.flush()?;
            res
        })))
    }

    pub fn set_async<K: Into<Vec<u8>>, V: Into<Vec<u8>>>(&self, key: K, value: V) -> impl Future<Item = (), Error = AngryError<()>> {
        let db = self.db.clone();
        let addr = self.db_sync_actor.clone();
        let k = key.into();
        let value = value.into();
        flatten_error!(poll_fn(move || blocking(|| {
            let res = db.set(k.clone(), value.clone());
            // Tell the DbSyncActor to flush the database
            addr.do_send(DbSyncCommand);
            return res;
        })))
    }

    pub fn get_async_u64<K: Into<Vec<u8>>>(&self, key: K) -> impl Future<Item = u64, Error = AngryError<()>> {
        self.get_async(key)
            .map(|r| r.unwrap_or(vec![0u8; 8]))
            .map(|r| LittleEndian::read_u64(&r))
    }

    pub fn set_async_u64<K: Into<Vec<u8>>>(&self, key: K, value: u64) -> impl Future<Item = (), Error = AngryError<()>> {
        let mut v = vec![0u8; 8];
        LittleEndian::write_u64(&mut v, value);
        self.set_async(key, v)
    }

    pub fn get_async_utf8<K: Into<Vec<u8>>>(&self, key: K) -> impl Future<Item = String, Error = AngryError<()>> {
        self.get_async(key)
            .map(|r| r.unwrap_or(vec![]))
            .and_then(|r| str::from_utf8(&r)
                .map(|s| s.to_owned())
                .map_err(|e| e.into()))
    }
}