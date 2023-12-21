//! Various structs handy for processing data between database and handlers' output

#[cfg(feature = "pgsql")]
pub mod as_map;

mod password;
pub use password::*;

pub mod role;

use serde::{Deserialize, Serialize};

/// Paged list ready to use in handler output. Produced by [pagination](crate::pagination) module.
#[derive(Serialize, Clone, Deserialize, Debug)]
#[cfg_attr(feature = "swagger", derive(paperclip::actix::Apiv2Schema))]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    pub total: i64,
    pub total_pages: i64,
    pub next_page: Option<i64>,
    pub data: Vec<T>,
}

/// Short for converting all `X` objects in `Vec` into `Y` objects in `Vec`
pub fn transmute<X, Y>(arg: Vec<X>) -> Vec<Y>
where
    X: Into<Y>,
{
    arg.into_iter().map(X::into).collect::<Vec<Y>>()
}

/// Short for converting all `X` objects in `Vec` into `Y` objects in `Vec`, but filter invalid ones by the way
pub fn filtermute<X, Y>(arg: Vec<X>) -> Vec<Y>
where
    X: Into<Option<Y>>,
{
    arg.into_iter().filter_map(X::into).collect::<Vec<Y>>()
}

pub mod cache_map;
