#![deny(clippy::all)]

#[cfg(feature = "pgsql")]
#[macro_use] extern crate diesel;

#[cfg(feature = "pgsql")]
#[macro_use] extern crate quick_error;

#[cfg(feature = "pgsql")]
pub mod auth;

pub mod containers;

#[cfg(feature = "pgsql")]
pub mod db_pool;

pub mod email;

pub mod server;

pub mod return_logged;

#[cfg(feature = "pgsql")]
pub mod role;

pub mod threads;

pub mod validation;

#[cfg(feature = "pgsql")]
pub mod pagination;

mod string_utils;

pub mod logger;

// Re-export EmptyStats from microservice_derive for convenience
pub use microservice_derive::EmptyStats;
