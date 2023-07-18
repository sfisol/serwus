use actix_web::{
    dev::ServiceResponse,
    http::{header, StatusCode},
    HttpResponse,
    middleware::ErrorHandlerResponse,
    ResponseError,
    Result,
};
use derive_more::Display;
use serde::{Serialize, Serializer};
use std::fmt::{Debug, Display};

#[derive(Debug, Display, Serialize)]
pub enum GenericErrorType<T> {
    BadRequest,
    NotFound,
    Internal,
    Other,
    ValidationFail,
    InvalidParams,
    Custom(T),
}

impl From<StatusCode> for GenericErrorType<()> {
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

#[derive(Debug, Display, Serialize)]
#[display(fmt = "{} ({}) {}", status_code, r#type, message)]
pub struct GenericError<T> {
    #[serde(serialize_with = "serialize_status_code")]
    pub status_code: StatusCode,
    pub r#type: GenericErrorType<T>,
    pub message: String,
}

impl<T> ResponseError for GenericError<T>
where
    T: Debug + Display + Serialize
{
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        if self.status_code > StatusCode::INTERNAL_SERVER_ERROR {
            log::error!("{}", self);
        }
        HttpResponse::build(self.status_code)
            .content_type("application/json; charset=utf-8")
            .json(self)
    }

    fn status_code(&self) -> StatusCode {
        self.status_code
    }
}

fn serialize_status_code<S>(value: &StatusCode, ser: S) -> Result<S::Ok, S::Error>
where S: Serializer
{
    ser.serialize_u16(value.as_u16())
}

pub fn default_error_handler<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let (req, mut response) = res.into_parts();

    let status_code = response.status();
    let r#type = GenericErrorType::from(status_code);
    let message = response.error().map(|err| err.to_string()).unwrap_or_else(|| status_code.to_string());

    let err = GenericError {
        status_code,
        r#type,
        message,
    };

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json; charset=utf-8"),
    );

    let res = response.set_body(serde_json::to_string(&err).unwrap()).map_into_boxed_body();
    let res = ServiceResponse::new(req, res).map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
}
