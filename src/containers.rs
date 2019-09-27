use ::serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    pub total_pages: i32,
    pub data: Vec<T>
}
