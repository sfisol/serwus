pub mod email;

#[cfg(feature = "auth")]
pub mod hash_password;

#[cfg(feature = "auth")]
mod generate_code;
#[cfg(feature = "auth")]
pub use generate_code::generate_code;

#[cfg(feature = "rabbit")]
pub mod rabbit;

pub mod validation;

mod string_utils;

mod sanitize;
pub use sanitize::*;

mod sanitized_string;
pub use sanitized_string::SanitizedString;

pub mod wrap_display;

#[cfg(feature = "actix-multipart")]
pub mod read_bytes;
