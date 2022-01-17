#![deny(rust_2018_idioms)]
#![deny(clippy::all)]

pub mod email;
pub mod validation;
#[cfg(feature = "pgsql")]
pub mod pagination;
mod string_utils;
pub mod wrap_display;
