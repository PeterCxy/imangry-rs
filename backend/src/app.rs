use byteorder::{ByteOrder, LittleEndian};
use futures::{Canceled, Future};
use futures::future::poll_fn;
use futures::sync::oneshot::channel;
use sled::{ConfigBuilder, Tree, Error as DbError};
use std::sync::Arc;
use tokio_threadpool::{BlockingError, blocking, ThreadPool};

#[derive(Debug)]
#[allow(dead_code)]
pub enum AngryError<E> {
    BlockingError(BlockingError),
    DbError(DbError<E>),
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
    pool: Arc<ThreadPool>
}

impl AngryAppState {
    pub fn new(db_path: String) -> AngryAppState {
        let config = ConfigBuilder::new()
            .path(db_path)
            .build();
        let db = Tree::start(config).unwrap();
        AngryAppState {
            db,
            pool: Arc::new(ThreadPool::new())
        }
    }

    pub fn get_db(&self) -> DbExt {
        DbExt {
            db: self.db.clone()
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

pub struct DbExt {
    db: Tree
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

    pub fn set_async<K: Into<Vec<u8>>>(&self, key: K, value: Vec<u8>) -> impl Future<Item = (), Error = AngryError<()>> {
        let db = self.db.clone();
        let k = key.into();
        flatten_error!(poll_fn(move || blocking(|| {
            db.set(k.clone(), value.clone())?;
            // TODO: Maybe we should not flush here?
            // Maybe we should flush it in a separate thread periodically?
            db.flush()
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
}