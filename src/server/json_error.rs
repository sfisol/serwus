use actix_http::body::MessageBody;
use actix_web::{
    HttpResponse, ResponseError, Result,
    dev::ServiceResponse,
    http::{StatusCode, header},
    middleware::ErrorHandlerResponse,
};
use serde::Serialize;
use std::fmt::{Debug, Display};

pub use serwus_derive::ResponseFromBuilder;

#[derive(Debug, derive_more::Display, Serialize)]
#[cfg_attr(feature = "swagger", derive(paperclip::actix::Apiv2Schema))]
pub enum JsonErrorType {
    BadRequest,
    NotFound,
    Internal,
    Other,
    Database,
    ValidationFail,
    InvalidParams,
    Custom(String),
}

impl From<StatusCode> for JsonErrorType {
    fn from(value: StatusCode) -> Self {
        if value == StatusCode::NOT_FOUND {
            Self::NotFound
        } else if value.is_client_error() {
            Self::BadRequest
        } else if value.is_server_error() {
            Self::Internal
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, derive_more::Display, Serialize)]
#[cfg_attr(feature = "swagger", derive(paperclip::actix::Apiv2Schema))]
#[display("{} ({}) {}", status_code, r#type, message)]
pub struct JsonError {
    // Skip `status_code` field to allow to derive Apiv2Schema without wrapping StatusCode type;
    // document and serialize field `status` instead.
    // Keep them private to assure consistency
    #[serde(skip)]
    status_code: StatusCode,
    status: u16,
    /// Generic type common to all services
    pub r#type: JsonErrorType,
    /// Message to be displayed to the user
    pub message: String,
    /// Error representation for tracing exact place in the code where error occurred
    pub debug: Option<String>,
    /// Detailed reason of the error
    pub reason: String,
    /// Any additional data, needs to be used when presenting error to the user (f. ex. validation errors)
    pub data: Option<serde_json::Value>,
}

pub const GENERIC_MESSAGE: &str = "Something went wrong. Try again later";
pub const GENERIC_REASON: &str = "Unknown";

pub struct ErrorBuilder {
    inner: JsonError,
}

impl ErrorBuilder {
    // Starters
    pub fn new(
        status_code: StatusCode,
        r#type: JsonErrorType,
        reason: impl Display,
    ) -> ErrorBuilder {
        let reason = reason.to_string();

        Self {
            inner: JsonError {
                status: status_code.as_u16(),
                status_code,
                r#type,
                message: GENERIC_MESSAGE.to_string(),
                debug: None,
                reason,
                data: None,
            },
        }
    }

    pub fn not_found() -> Self {
        Self::not_found_msg(StatusCode::NOT_FOUND.to_string())
    }

    pub fn not_found_msg(message: impl Display) -> Self {
        let status_code = StatusCode::NOT_FOUND;
        let message = message.to_string();
        Self::new(status_code, JsonErrorType::NotFound, message.clone()).message(message)
    }

    pub fn bad_request(reason: impl Display) -> Self {
        let status_code = StatusCode::BAD_REQUEST;
        Self::new(status_code, JsonErrorType::BadRequest, reason)
    }

    pub fn internal(reason: impl Display) -> Self {
        let status_code = StatusCode::INTERNAL_SERVER_ERROR;
        Self::new(status_code, JsonErrorType::Internal, reason)
    }

    pub fn database(reason: impl Display) -> Self {
        let status_code = StatusCode::INTERNAL_SERVER_ERROR;
        Self::new(status_code, JsonErrorType::Database, reason)
    }

    pub fn unauthorized(reason: impl Display) -> Self {
        let status_code = StatusCode::UNAUTHORIZED;
        Self::new(status_code, JsonErrorType::Other, reason)
    }

    pub fn forbidden(reason: impl Display) -> Self {
        let status_code = StatusCode::FORBIDDEN;
        Self::new(status_code, JsonErrorType::Other, reason)
    }

    pub fn validation_fail(reason: impl Display) -> Self {
        let status_code = StatusCode::UNPROCESSABLE_ENTITY;
        Self::new(status_code, JsonErrorType::ValidationFail, reason)
    }

    pub fn validation_fail_msg(message: impl Display) -> Self {
        let message = message.to_string();
        Self::validation_fail(message.clone()).message(message)
    }

    pub fn custom(sub_type: impl Display, reason: impl Display) -> Self {
        let status_code = StatusCode::INTERNAL_SERVER_ERROR;
        Self::new(
            status_code,
            JsonErrorType::Custom(sub_type.to_string()),
            reason,
        )
    }

    // Modifiers
    pub fn status(mut self, status_code: StatusCode) -> Self {
        self.inner.status = status_code.as_u16();
        self.inner.status_code = status_code;
        self
    }

    pub fn message(mut self, message: impl Display) -> Self {
        self.inner.message = message.to_string();
        self
    }

    pub fn r#type(mut self, r#type: JsonErrorType) -> Self {
        self.inner.r#type = r#type;
        self
    }

    pub fn debug(mut self, debug: impl Debug) -> Self {
        self.inner.debug = Some(format!("{debug:?}"));
        self
    }

    pub fn data(mut self, data: impl Serialize) -> Self {
        let data = serde_json::to_value(&data)
            .or_else(|err| {
                let err_str = err.to_string();
                log::error!("Error serializing error: {err_str}");
                Ok::<_, ()>(serde_json::Value::String(format!(
                    "Error serializing error: {err_str}"
                )))
            })
            .ok();

        self.inner.data = data;
        self
    }

    // Produce ready error
    pub fn finish(self) -> JsonError {
        self.inner
    }
}

impl ResponseError for JsonError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        if self.status_code > StatusCode::INTERNAL_SERVER_ERROR {
            log::error!("{} (reason: {})", self, self.reason);
        }
        HttpResponse::build(self.status_code)
            .content_type("application/json; charset=utf-8")
            .json(self)
    }

    fn status_code(&self) -> StatusCode {
        self.status_code
    }
}

/// Middleware for converting plain actix errors into JSON ones with JsonError schema
pub fn default_error_handler<B: MessageBody + 'static>(
    res: ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>> {
    // Disassemble service response
    let (req, mut response) = res.into_parts();

    let json_content_type = header::HeaderValue::from_static("application/json; charset=utf-8");

    // Rewrite response only if it is an error, but not JSON already
    let res = if !response.status().is_success()
        && response.headers().get(header::CONTENT_TYPE) != Some(&json_content_type)
    {
        let status_code = response.status();
        let r#type = JsonErrorType::from(status_code);

        // Happily error message can be taken from "special place" in response struct,
        // not necessarily from the body which requires waiting.
        // If error message is not provided, take it from status code.
        let error = response.error();

        // TODO: Maybe sometimes it would be better to place this in 'reason' but how to distinguish when?
        let message = error
            .map(|err| err.to_string())
            .unwrap_or_else(|| status_code.to_string());

        let debug = error
            .map(|e| format!("{e:?}"))
            .unwrap_or_else(|| "default_error_handler".to_string());

        let err = JsonError {
            status: status_code.as_u16(),
            status_code,
            r#type,
            message,
            debug: Some(debug),
            reason: "".to_string(),
            data: None,
        };

        // Overwrite response content-type
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, json_content_type);

        // Overwrite response body
        response
            .set_body(serde_json::to_string(&err).unwrap())
            .map_into_boxed_body()
    } else {
        // Leave response intact
        response.map_into_boxed_body()
    };

    let res = ServiceResponse::new(req, res).map_into_right_body();
    Ok(ErrorHandlerResponse::Response(res))
}
