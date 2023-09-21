//! Serwus is a set of helpers for building actix-web/diesel based services.
//!
//! ## Features
//!
//! * **[MultiPool](db_pool::multi::MultiPool)** - Master/replica-aware wrapper for `r2d2`
//! * **[StatsPresenter](server::stats::StatsPresenter)** - Framework for readiness and statistics reporting
//! * **[JsonError](server::json_error::JsonError)** - Middleware that makes actix-web return errors as JSONs
//!
//! ## Example
//!
//! ```no_run
//! use serwus::{
//!     server::{Serwus, default_cors},
//!     EmptyStats,
//!     web,
//! };
//!
//! #[derive(Clone, EmptyStats)]
//! pub struct AppData;
//!
//! # #[cfg_attr(feature = "swagger", paperclip::actix::api_v2_operation)]
//! async fn hello() -> &'static str {
//!     "Hello world\n"
//! }
//!
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     let prepare_app_data = || AppData;
//!
//!     Serwus::default()
//!         .start(
//!             prepare_app_data,
//!             |app| {
//!                 app.route("/", web::get().to(hello));
//!             },
//!             default_cors,
//!         )
//!         .await
//! }
//! ```


#![deny(clippy::all)]

#![feature(result_flattening)]

pub mod containers;
pub mod utils;

pub mod auth;
#[cfg(feature = "pgsql")]
pub mod db_pool;

pub mod server;

pub mod return_logged;

pub mod threads;

#[cfg(feature = "pgsql")]
pub mod pagination;

pub mod logger;

/// Re-export of `web` from `actix-web` or from `paperclip` if swagger feature enabled.
#[cfg(feature = "swagger")]
pub use paperclip::actix::web;
#[cfg(not(feature = "swagger"))]
pub use actix_web::web;

/// Automatic implementation of [StatsPresenter](server::stats::StatsPresenter)
///
/// Returns `()` as stats, and always repors as ready.
pub use serwus_derive::EmptyStats;
