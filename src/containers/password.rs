use derive_more::Display;
use serde::{Deserialize, Serializer, Serialize};
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize, Display, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "pgsql", derive(diesel_derive_newtype::DieselNewType))]
#[cfg_attr(feature = "paperclip", derive(paperclip::actix::Apiv2Schema))]
pub struct Password(pub String);

impl Serialize for Password {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
            S: Serializer
    {
        serializer.serialize_str("******")
    }
}

impl Deref for Password {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Password {
    fn from(value: String) -> Self {
        Self(value)
    }
}
