use actix_web::{web, error::BlockingError};
use diesel::pg::PgConnection;
use diesel::Connection;
use r2d2::{self, Error};
use r2d2_diesel::ConnectionManager;
use std::env;
use log::{error, info};

use super::threads::num_threads;

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

    #[allow(clippy::cast_possible_truncation)]
    Pool::builder()
        .max_size(max_size as u32)
        .build(manager)
        .map_err(|err| {
            error!("Can't connect to database: {}", err);
            err
        })
}

fn database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

quick_error! {
    #[derive(Debug)]
    pub enum AsyncQueryError {
        Conn(err: r2d2::Error) { from() }
        Query(err: diesel::result::Error) { from() }
    }
}

pub type AsyncQueryResult<R> = Result<R, BlockingError<AsyncQueryError>>;

pub async fn async_query<R: Send + 'static, F: Send + 'static>(db_pool: Pool, query_func: F) -> Result<R, BlockingError<AsyncQueryError>>
where
    F: FnOnce(&PgConnection) -> diesel::result::QueryResult<R>
{
    web::block(move || {
        let connection = db_pool.get()?;
        query_func(&connection)
            .map_err(Into::into)
    }).await
}

pub async fn async_transaction<R: Send + 'static, F: Send + 'static>(db_pool: Pool, query_func: F) -> Result<R, BlockingError<AsyncQueryError>>
where
    F: FnOnce(&PgConnection) -> diesel::result::QueryResult<R>
{
    web::block(move || {
        let connection = db_pool.get()?;
        connection.transaction(||
            query_func(&connection)
                .map_err(Into::into)
        )
    }).await
}

pub async fn async_query_in_trans<R: Send + 'static, F: Send + 'static>(connection: PgConnection, query_func: F) -> Result<R, BlockingError<diesel::result::Error>>
where
    F: FnOnce(&PgConnection) -> diesel::result::QueryResult<R>,
    R: std::fmt::Debug
{
    web::block(move || {
        query_func(&connection)
    }).await
}
