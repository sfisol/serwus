#![deny(rust_2018_idioms)]
#![warn(clippy::all)]

#[cfg(feature = "pgsql")]
pub mod as_map;

mod password;
pub use password::*;

pub mod role;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Clone, Deserialize, Debug)]
#[cfg_attr(feature = "swagger", derive(paperclip::actix::Apiv2Schema))]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    pub total: i64,
    pub total_pages: i64,
    pub data: Vec<T>
}

pub fn transmute<X, Y>(arg: Vec<X>) -> Vec<Y>
where
    X: Into<Y>
{
    arg.into_iter().map(X::into).collect::<Vec<Y>>()
}

pub fn filtermute<X, Y>(arg: Vec<X>) -> Vec<Y>
where
    X: Into<Option<Y>>
{
    arg.into_iter().filter_map(X::into).collect::<Vec<Y>>()
}
