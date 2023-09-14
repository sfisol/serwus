//! Helpers for creating r2d2 pool based on ENV variables and number of CPUs.

mod single;
pub use single::*;

#[cfg(feature = "multidb")]
pub mod multi;

mod async_queries;
pub use async_queries::*;
