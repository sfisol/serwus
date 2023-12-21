use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize};
use validator::HasLen;

/// Wrapper for String that gets automatically trimmed during deserialization
#[cfg_attr(feature = "swagger", derive(paperclip::actix::Apiv2Schema))]
#[derive(
    Serialize,
    derive_more::Deref,
    derive_more::Into,
    derive_more::Display,
    derive_more::AsRef,
    Clone,
    Eq,
    PartialEq,
    Default,
)]
#[as_ref(forward)]
pub struct SanitizedString(String);

impl<'de> Deserialize<'de> for SanitizedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Cow<'de, str> = Deserialize::deserialize(deserializer)?;
        let s = s.trim().to_owned();

        Ok(Self(s))
    }
}

impl<'a> From<&'a SanitizedString> for Cow<'a, str> {
    fn from(value: &'a SanitizedString) -> Self {
        value.0.as_str().into()
    }
}

impl From<String> for SanitizedString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl HasLen for &SanitizedString {
    fn length(&self) -> u64 {
        self.0.length()
    }
}

#[cfg(test)]
mod tests {
    use validator::Validate;
    use validator_derive::Validate;

    use super::*;

    #[derive(Deserialize, Validate)]
    struct Foo {
        #[validate(length(min = 1, max = 128))]
        name: SanitizedString,
    }

    #[test]
    fn test_trimming() {
        let json = r#"{"name":" test "}"#;
        let foo = serde_json::from_str::<Foo>(json).unwrap();

        assert_eq!(foo.name.to_string(), "test");
    }

    #[test]
    fn test_escaped_chars() {
        let json = r#"{"name":" test1 \n test1\b "}"#;
        let foo = serde_json::from_str::<Foo>(json).unwrap();

        assert_eq!(foo.name.to_string(), "test1 \n test1\u{8}");
    }

    #[test]
    fn test_validation() {
        let json = r#"{"name":""}"#;
        let foo = serde_json::from_str::<Foo>(json).unwrap();

        let result = foo.validate();
        assert!(result.is_err())
    }
}
