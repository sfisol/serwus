use actix_web::web;
use diesel::{
    Connection,
    pg::PgConnection,
};

#[cfg(not(feature = "multidb"))]
use super::Pool;

#[cfg(feature = "multidb")]
use super::multi::MultiPool;

// Re-export Canceled from microservice_derive for convenience
pub use microservice_derive::Canceled;

#[cfg(not(feature = "multidb"))]
pub async fn async_query<F, I, E>(db_pool: Pool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.get()?;
        query_func(&connection)
            .map_err(From::from)
    })
        .await
        .map_err(From::from)
        .flatten()
}

#[cfg(not(feature = "multidb"))]
pub async fn async_transaction<F, I, E>(db_pool: Pool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&mut PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + From<diesel::result::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let mut connection = db_pool.get()?;
        connection.transaction(|connection|
            query_func(connection)
                .map_err(From::from)
        )
    })
        .await
        .map_err(From::from)
        .flatten()
}

pub async fn async_query_in_trans<F, I, E>(connection: PgConnection, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        query_func(&connection)
    })
        .await
        .map_err(From::from)
        .flatten()
}

#[cfg(feature = "multidb")]
/// Keep in mind that this function does not perform a read only transaction,
/// so write queries can be performed if provided database allows it.
/// Use async_read_transaction to perform query in real read-only mode.
pub async fn async_read_query<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.read()?;
        query_func(&connection)
            .map_err(From::from)
    })
        .await
        .map_err(From::from)
        .flatten()
}

#[cfg(feature = "multidb")]
pub async fn async_write_query<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.write()?;
        query_func(&connection)
            .map_err(From::from)
    })
        .await
        .map_err(From::from)
        .flatten()
}

#[cfg(feature = "multidb")]
pub async fn async_read_transaction<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + From<diesel::result::Error> + std::fmt::Debug + Send + 'static,
{
    use diesel::{RunQueryDsl, sql_query};

    web::block(move || {
        let mut connection = db_pool.read()?;
        connection.transaction(|connection| {
            sql_query("SET TRANSACTION READ ONLY").execute(connection)?;
            query_func(connection)
                .map_err(From::from)
        })
    })
        .await
        .map_err(From::from)
        .flatten()
}

#[cfg(feature = "multidb")]
pub async fn async_write_transaction<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError> + From<r2d2::Error> + From<diesel::result::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let mut connection = db_pool.write()?;
        connection.transaction(|connection|
            query_func(connection)
                .map_err(From::from)
        )
    })
        .await
        .map_err(From::from)
        .flatten()
}
