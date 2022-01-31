#![deny(rust_2018_idioms)]
#![deny(clippy::all)]

pub mod email;

#[cfg(feature = "auth")]
pub mod hash_password;

mod generate_code;
pub use generate_code::generate_code;

#[cfg(feature = "rabbit")]
pub mod rabbit;

pub mod validation;

mod string_utils;

mod sanitize;
pub use sanitize::*;

pub mod wrap_display;
