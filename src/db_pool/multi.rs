//! Pool made of pools, one writable and others read-only with connections to slave replica(s).

use std::{
    env,
    sync::{Arc, Mutex},
};

use diesel::r2d2::ConnectionManager;
use log::{error, info};
use r2d2::{Error, PooledConnection};
use weighted_rs::{RoundrobinWeight, Weight};

use crate::threads::num_threads;

use super::{database_url, Pool, DbConnection};

/// Pool made of pools, one writable and others read-only with connections to slave replica(s).
///
/// Example:
///
/// ```
/// use serwus::db_pool::multi::{MultiPool, MultiPoolBuilder};
///
/// pub struct AppData {
///    db_pool: MultiPool,
/// }
///
/// impl Default for AppData {
///    fn default() -> Self {
///       Self {
///          // Use DATABASE_URL env for writable database
///          // Use DATABASE_MIRRORS_URLS env for read-only databases (comma-separated)
///          db_pool: MultiPoolBuilder::default()
///             .connect()
///             .expect("Can't connect to databases")
///       }
///    }
/// }
/// ```
#[derive(Clone)]
pub struct MultiPool {
    master: Option<Pool>,
    mirrors: Vec<Pool>,
    dispatcher: Arc<Mutex<RoundrobinWeight<usize>>>,
}

#[derive(Debug)]
pub enum InitMultiError {
    MasterFailed(Error),
    MirrorFailed((String, Error)),
}

pub struct MultiPoolBuilder<'a> {
    size: usize,
    write_url_env: &'a str,
    read_url_env: &'a str,
    read_only: bool,
}

impl Default for MultiPoolBuilder<'_> {
    fn default() -> Self {
        let nthreads = num_threads();
        Self {
            size: if nthreads > 1 { nthreads } else { 2 },
            write_url_env: "DATABASE_URL",
            read_url_env: "DATABASE_MIRRORS_URLS",
            read_only: false,
        }
    }
}

impl<'a> MultiPoolBuilder<'a> {
    #[must_use]
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn write_url_env(mut self, write_url_env: &'a str) -> Self {
        self.write_url_env = write_url_env;
        self
    }

    #[must_use]
    pub fn read_url_env(mut self, read_url_env: &'a str) -> Self {
        self.read_url_env = read_url_env;
        self
    }

    #[must_use]
    pub fn readonly(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn connect(self) -> Result<MultiPool, InitMultiError> {
        info!("Connecting to database(s)");

        let max_size = if env::var("TEST").is_ok() && self.size > 2 {
            2
        } else {
            self.size
        };

        let master = if self.read_only {
            None
        } else {
            let manager = ConnectionManager::<DbConnection>::new(database_url(self.write_url_env));

            #[allow(clippy::cast_possible_truncation)]
            Some(
                Pool::builder()
                    .max_size(max_size as u32)
                    .build(manager)
                    .map_err(|err| {
                        error!("Can't connect to database: {}", err);
                        InitMultiError::MasterFailed(err)
                    })?,
            )
        };

        let mut mirrors = vec![];
        let mut dispatcher = RoundrobinWeight::new();

        for url in database_mirrors_urls(self.read_url_env) {
            let manager = ConnectionManager::<DbConnection>::new(url.clone());

            mirrors.push(
                Pool::builder()
                    .max_size(max_size as u32)
                    .build(manager)
                    .map_err(|err| {
                        error!("Can't connect to database: {}", err);
                        InitMultiError::MirrorFailed((url, err))
                    })?,
            );

            dispatcher.add(mirrors.len() - 1, 1);
        }

        if self.read_only {
            info!(
                "Initialized read only pool with {} nodes with {} conns each",
                mirrors.len(),
                max_size
            );
        } else {
            info!(
                "Initialized writable pool with {} read mirror(s) with {} conns each",
                mirrors.len(),
                max_size
            );
        }

        Ok(MultiPool {
            master,
            mirrors,
            dispatcher: Arc::new(Mutex::new(dispatcher)),
        })
    }
}

impl MultiPool {
    pub fn write(&self) -> Result<PooledConnection<ConnectionManager<DbConnection>>, Error> {
        self.master.as_ref().expect("Readonly database pool").get()
    }

    pub fn read(&self) -> Result<PooledConnection<ConnectionManager<DbConnection>>, Error> {
        let n_opt = match self.dispatcher.lock() {
            Ok(mut dispatcher) => dispatcher.next(),
            Err(_) => {
                error!("Error acquiring mirrors mutex, returning master db connection");
                None
            }
        };

        if let Some(n) = n_opt {
            self.mirrors[n].get()
        } else {
            self.write()
        }
    }
}

fn database_mirrors_urls(env_name: &str) -> Vec<String> {
    env::var(env_name)
        .map(|value| value.split(',').map(String::from).collect())
        .unwrap_or_default()
}
