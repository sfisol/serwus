#[cfg(feature = "actix_validation")]
use actix_web::error;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
};
use validator::ValidationErrors;
#[cfg(feature = "actix_validation")]
use validator::ValidationErrorsKind;

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
                                    code: val.code.clone().to_string(),
                                    message: v.clone().to_string(),
                                }
                            ),
                            None => x.push(
                                Error {
                                    code: val.code.clone().to_string(),
                                    message: "".to_string(),
                                }
                            )
                        }
                    }

                } else {
                    let message = match &val.message {
                        Some(v) => v.clone().to_string(),
                        None => "".to_string()
                    };

                    let errors: Vec<Error> = vec![
                        Error {
                            code: val.code.clone().to_string(),
                            message,
                        }
                    ];

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

#[cfg(feature = "actix_validation")]
pub fn render_single_validation_error(err: &ValidationErrors) -> error::Error {
    match err.to_owned().errors().values().next() {
        Some(ValidationErrorsKind::Field(field_errors)) => {
            if let Some(field_error) = field_errors.iter().next() {
                let code = field_error.code.clone().into_owned();
                match field_error.message.clone() {
                    Some(message) => error::ErrorBadRequest(message),
                    None => error::ErrorInternalServerError(format!("No validation message for: {}", code)),
                }
            } else { error::ErrorInternalServerError("Empty field errors".to_string()) }
        },
        _ => error::ErrorInternalServerError("Malformed validation".to_string()),
    }
}
