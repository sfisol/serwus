use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager};
use std::env;
use log::{error, info};

use crate::threads::num_threads;

pub type Pool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_default_pool() -> Result<Pool, r2d2::Error> {
    let nthreads = num_threads();
    init_pool(if nthreads > 1 { nthreads } else { 2 })
}

pub fn init_pool(size: usize) -> Result<Pool, r2d2::Error> {
    info!("Connecting to database");

    let manager = ConnectionManager::<PgConnection>::new(default_database_url());

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

pub(super) fn database_url(env_name: &str) -> String {
    env::var(env_name).unwrap_or_else(|_| format!("{env_name} must be set"))
}

fn default_database_url() -> String {
    database_url("DATABASE_URL")
}
