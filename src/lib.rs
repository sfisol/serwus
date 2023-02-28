#![deny(clippy::all)]

#![feature(result_flattening)]

pub mod auth;
#[cfg(feature = "pgsql")]
pub mod db_pool;

pub mod server;

pub mod return_logged;

pub mod threads;

#[cfg(feature = "pgsql")]
pub mod pagination;

pub mod logger;

// Re-export EmptyStats from microservice_derive for convenience
pub use microservice_derive::EmptyStats;
