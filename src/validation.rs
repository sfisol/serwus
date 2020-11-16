use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
};
use validator::ValidationErrors;

use super::string_utils::to_camel_case;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeError {
    ValidationCodeError = 0,
}

impl CodeError {
    fn as_num(&self) -> i16 {
        match *self {
            CodeError::ValidationCodeError => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    code: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    code: i16,
    errors: HashMap<String, Vec<Error>>,
}

pub fn return_new_error(code: &'static str, message: &'static str) -> validator::ValidationError {
    return_new_dynamic_error(code, Cow::Borrowed(message))
}

pub fn return_new_dynamic_error(code: &'static str, message: Cow<'static, str>) -> validator::ValidationError {
    let mut error = validator::ValidationError::new(code);
    error.message = Some(message);
    error
}

impl ValidationError {

    pub fn from(val_errors: &ValidationErrors) -> Self {

        let mut validation_error = Self {
            code: CodeError::as_num(&CodeError::ValidationCodeError),
            errors: HashMap::new(),
        };

        for (field_key, field_val) in val_errors.field_errors().iter() {
            for val in field_val.iter() {
                if validation_error.errors.contains_key(*field_key) {
                    if let Some(x) = validation_error.errors.get_mut(*field_key) {
                        match &val.message {
                            Some(v) => x.push(
                                Error {
                                    code: val.code.to_owned().to_string(),
                                    message: v.to_owned().to_string(),
                                }
                            ),
                            None => x.push(
                                Error {
                                    code: val.code.to_owned().to_string(),
                                    message: "".to_owned(),
                                }
                            )
                        }
                    }

                } else {
                    let message = match &val.message {
                        Some(v) => v.to_owned().to_string(),
                        None => "".to_owned()
                    };

                    let mut errors: Vec<Error> = Vec::new();
                    errors.push(
                        Error {
                            code: val.code.to_owned().to_string(),
                            message,
                        }
                    );

                    validation_error.errors.insert(
                        to_camel_case(field_key),
                        errors,
                    );
                }
            }
        };

        validation_error
    }
}
