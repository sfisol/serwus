use diesel::pg::PgConnection;
use r2d2::{self, Error};
use r2d2_diesel::ConnectionManager;
use std::env;
use log::{error, info};

use crate::threads::num_threads;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_default_pool() -> Result<Pool, Error> {
    let nthreads = num_threads();
    init_pool(if nthreads > 1 { nthreads } else { 2 })
}

pub fn init_pool(size: usize) -> Result<Pool, Error> {
    info!("Connecting to database");

    let manager = ConnectionManager::<PgConnection>::new(database_url());

    let max_size = if env::var("TEST").is_ok() && size > 2 {
        2
    } else {
        size
    };

    // #[allow(clippy::cast_possible_truncation)]
    Pool::builder()
        .max_size(max_size as u32)
        .build(manager)
        .map_err(|err| {
            error!("Can't connect to database: {}", err);
            err
        })
}

pub(super) fn database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}
