//! Few helpers for spawning and configuring service in actix ecosystem.

pub mod app_data;
mod builder;
pub mod json_error;
#[cfg(feature = "prometheus")]
pub mod prometheus;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod stats;

use actix_cors::Cors;
use actix_http::Request;
use actix_service::Service;
use actix_web::{
    App, http, test, Error,
    body::BoxBody,
    dev::ServiceResponse,
};

#[cfg(not(feature = "swagger"))]
use actix_web::web;

#[cfg(feature = "swagger")]
use paperclip::actix::{web, OpenApiExt};

use super::threads;
use super::logger;

pub use app_data::{DefaultAppData, default_app_data};
pub use builder::Serwus;

/// Use this Cors builder if you want to send wildcard and allow default set ot http methods and headers
pub fn default_cors() -> Cors {
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
        .max_age(3600)
}

pub async fn test_init<T, F>(prepare_app_data: impl Fn() -> T, configure_app: F) -> impl Service<Request, Response = ServiceResponse<BoxBody>, Error = Error>
where
    T: 'static,
    F: Fn(&mut web::ServiceConfig),
{
    let app_data = prepare_app_data();

    #[allow(clippy::let_and_return)]
    test::init_service({
        let app = App::new();

        #[cfg(feature = "swagger")]
        let app = app.wrap_api();

        let app = app
            .app_data(app_data)
            .configure(configure_app);

        #[cfg(feature = "swagger")]
        let app = app.build();

        app
    }).await
}
