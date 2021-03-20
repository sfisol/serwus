use actix_web::web;
use diesel::pg::PgConnection;
use diesel::Connection;

use super::Pool;

#[cfg(feature = "multidb")]
use super::multi::MultiPool;

// Re-export Canceled from microservice_derive for convenience
pub use microservice_derive::Canceled;

pub async fn async_query<F, I, E>(db_pool: Pool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError<E>> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.get()?;
        query_func(&connection)
            .map_err(Into::into)
    })
        .await
        .map_err(From::from)
}

pub async fn async_transaction<F, I, E>(db_pool: Pool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError<E>> + From<r2d2::Error> + From<diesel::result::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.get()?;
        connection.transaction(||
            query_func(&connection)
                .map_err(Into::into)
        )
    })
        .await
        .map_err(From::from)
}

pub async fn async_query_in_trans<F, I, E>(connection: PgConnection, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError<E>> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        query_func(&connection)
    })
        .await
        .map_err(From::from)
}

#[cfg(feature = "multidb")]
pub async fn async_read_query<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError<E>> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.read()?;
        query_func(&connection)
            .map_err(Into::into)
    })
        .await
        .map_err(From::from)
}

#[cfg(feature = "multidb")]
pub async fn async_write_query<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError<E>> + From<r2d2::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.write()?;
        query_func(&connection)
            .map_err(Into::into)
    })
        .await
        .map_err(From::from)
}

#[cfg(feature = "multidb")]
pub async fn async_write_transaction<F, I, E>(db_pool: MultiPool, query_func: F) -> Result<I, E>
where
    F: FnOnce(&PgConnection) -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: From<actix_web::error::BlockingError<E>> + From<r2d2::Error> + From<diesel::result::Error> + std::fmt::Debug + Send + 'static,
{
    web::block(move || {
        let connection = db_pool.write()?;
        connection.transaction(||
            query_func(&connection)
                .map_err(Into::into)
        )
    })
        .await
        .map_err(From::from)
}
