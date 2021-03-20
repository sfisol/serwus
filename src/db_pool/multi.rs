use diesel::pg::PgConnection;
use r2d2::{self, Error, PooledConnection};
use r2d2_diesel::ConnectionManager;
use std::env;
use log::{error, info};
use weighted_rs::{Weight, RoundrobinWeight};
use std::sync::{Arc, Mutex};

use crate::threads::num_threads;

use super::{Pool, database_url};

#[derive(Clone)]
pub struct MultiPool {
    master: Pool,
    mirrors: Vec<Pool>,
    dispatcher: Arc<Mutex<RoundrobinWeight<usize>>>,
}

#[derive(Debug)]
pub enum InitMultiError {
    MasterFailed(Error),
    MirrorFailed((String, Error)),
}

pub fn init_default_multi_pool() -> Result<MultiPool, InitMultiError> {
    let nthreads = num_threads();
    init_multi_pool(if nthreads > 1 { nthreads } else { 2 })
}

pub fn init_multi_pool(size: usize) -> Result<MultiPool, InitMultiError> {
    info!("Connecting to database(s)");

    let max_size = if env::var("TEST").is_ok() && size > 2 {
        2
    } else {
        size
    };

    let manager = ConnectionManager::<PgConnection>::new(database_url());

    #[allow(clippy::cast_possible_truncation)]
    let master = Pool::builder()
        .max_size(max_size as u32)
        .build(manager)
        .map_err(|err| {
            error!("Can't connect to database: {}", err);
            InitMultiError::MasterFailed(err)
        })?;

    let mut mirrors = vec![];
    let mut dispatcher = RoundrobinWeight::new();

    for url in database_mirrors_urls() {
        let manager = ConnectionManager::<PgConnection>::new(url.clone());

        mirrors.push(
            Pool::builder()
                .max_size(max_size as u32)
                .build(manager)
                .map_err(|err| {
                    error!("Can't connect to database: {}", err);
                    InitMultiError::MirrorFailed((url, err))
                })?
        );

        dispatcher.add(mirrors.len() - 1, 1);
    }

    info!("Initialized pool with {} mirror(s) with {} conns each", mirrors.len(), max_size);

    Ok(
        MultiPool {
            master,
            mirrors,
            dispatcher: Arc::new(Mutex::new(dispatcher))
        }
    )
}

impl MultiPool {
    pub fn write(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
        self.master.get()
    }

    pub fn read(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
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

fn database_mirrors_urls() -> Vec<String> {
    env::var("DATABASE_MIRRORS_URLS")
        .map(|value|
            value.split(',')
                .map(String::from)
                .collect()
        )
        .unwrap_or_default()
}
