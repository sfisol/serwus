#[cfg(feature = "pgsql")]
pub mod as_map;

#[cfg(feature = "swagger")]
use paperclip::actix::Apiv2Schema;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "swagger", derive(Apiv2Schema))]
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
