//! Helpers for end-to-end tests of serwus services.
//!
//! - [`postgres`]: Postgres in a container, and a client for setting up data and checking what
//!   the service wrote,
//! - [`backend`]: the service itself, run as a separate process with the settings serwus reads,
//!   until it answers its healthcheck.

pub mod backend;
pub mod postgres;

pub use backend::Backend;
pub use postgres::{Db, PgSpec, Postgres};

pub use tokio_postgres;
