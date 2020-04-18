#[cfg(feature = "pgsql")]
pub mod as_map;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
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
