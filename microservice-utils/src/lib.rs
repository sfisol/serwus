#![deny(clippy::all)]

// #![feature(result_flattening)]

// pub mod auth;

pub mod email;

// pub mod return_logged;

// pub mod threads;

pub mod validation;

#[cfg(feature = "pgsql")]
pub mod pagination;

mod string_utils;

// pub mod logger;

pub mod wrap_display;
