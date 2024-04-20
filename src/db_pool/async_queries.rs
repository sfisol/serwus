use std::convert;

use actix_web::web;
use diesel::Connection;

use super::DbConnection;

#[cfg(not(feature = "multidb"))]
use super::Pool;

#[cfg(feature = "multidb")]
use super::multi::MultiPool;

// Re-export Canceled from serwus_derive for convenience
pub use serwus_derive::Canceled;

/// Performs query to database as blocking task
#[cfg(not(feature = "multidb"))]
pub async fn async_query<F, I, E>(db_pool: Pool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&mut DbConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let mut connection = db_pool.get()?;
        query_func(&mut connection).map_err(From::from)
    })
    .await
    .map_err(From::from)
    .and_then(convert::identity)
}

/// Performs one or more queries to database in transaction (as blocking task)
#[cfg(not(feature = "multidb"))]
pub async fn async_transaction<F, I, E>(db_pool: Pool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&mut DbConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError>
        + From<r2d2::Error>
        + From<diesel::result::Error>
        + std::fmt::Debug
        + Send
        + 'static,
{
    web::block(move || {
        let mut connection = db_pool.get()?;
        connection.transaction(|connection| query_func(connection).map_err(From::from))
    })
    .await
    .map_err(From::from)
    .and_then(convert::identity)
}

/// Performs query to database in currently open transaction (as blocking task)
pub async fn async_query_in_trans<F, I, E>(
    mut connection: DbConnection,
    query_func: F,
) -> Result<I, E>
where
    F: FnOnce(&mut DbConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || query_func(&mut connection))
        .await
        .map_err(From::from)
        .and_then(convert::identity)
}

#[cfg(feature = "multidb")]
/// Perform read query to one of databases (master or replica(s)) as blocking task.
///
/// Keep in mind that this function does not perform a read only transaction,
/// so write queries can be performed if provided database allows it.
/// Use async_read_transaction to perform query in real read-only mode.
pub async fn async_read_query<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&mut DbConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let mut connection = db_pool.read()?;
        query_func(&mut connection).map_err(From::from)
    })
    .await
    .map_err(From::from)
    .and_then(convert::identity)
}

/// Perform read/write query to master database as blocking task.
#[cfg(feature = "multidb")]
pub async fn async_write_query<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&mut DbConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let mut connection = db_pool.write()?;
        query_func(&mut connection).map_err(From::from)
    })
    .await
    .map_err(From::from)
    .and_then(convert::identity)
}

/// Perform one or more read queries to one of databases (master or replica(s)) as blocking task.
#[cfg(feature = "multidb")]
pub async fn async_read_transaction<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&mut DbConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError>
        + From<r2d2::Error>
        + From<diesel::result::Error>
        + std::fmt::Debug
        + Send
        + 'static,
{
    use diesel::{sql_query, RunQueryDsl};

    web::block(move || {
        let mut connection = db_pool.read()?;
        connection.transaction(|connection| {
            sql_query("SET TRANSACTION READ ONLY").execute(connection)?;
            query_func(connection).map_err(From::from)
        })
    })
    .await
    .map_err(From::from)
    .and_then(convert::identity)
}

/// Perform read/write query to master database as blocking task.
#[cfg(feature = "multidb")]
pub async fn async_write_transaction<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&mut DbConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError>
        + From<r2d2::Error>
        + From<diesel::result::Error>
        + std::fmt::Debug
        + Send
        + 'static,
{
    web::block(move || {
        let mut connection = db_pool.write()?;
        connection.transaction(|connection| query_func(connection).map_err(From::from))
    })
    .await
    .map_err(From::from)
    .and_then(convert::identity)
}
