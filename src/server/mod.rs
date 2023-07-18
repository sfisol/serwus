pub mod app_data;
mod builder;
pub mod json_error;
#[cfg(feature = "prometheus")]
pub mod prometheus;
pub mod stats;

use actix_cors::Cors;
use actix_http::Request;
use actix_service::Service;
use actix_web::{
    App, http, test, Error,
    body::BoxBody,
    dev::ServiceResponse,
};
use std::io;

#[cfg(not(feature = "swagger"))]
use actix_web::web;

#[cfg(feature = "swagger")]
use paperclip::actix::{web, OpenApiExt};

use super::threads;

#[cfg(feature = "pgsql")]
use super::db_pool;

use super::logger;
use stats::{StatsPresenter, AppDataWrapper};

pub use app_data::{DefaultAppData, default_app_data};
pub use builder::Microservice;

fn default_cors() -> Cors {
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
        .max_age(3600)
}

#[deprecated]
pub async fn start<D, T, F>
(
    prepare_app_data: impl Fn() -> T,
    configure_app: F,
    app_port: &str,
    run_env: &str,
) -> io::Result<()>
where
    D: AppDataWrapper + 'static,
    T: StatsPresenter<D> + 'static + Clone + Send + Sync,
    F: Fn(&mut web::ServiceConfig) + Send + Clone + Copy + 'static,
{
    #[allow(deprecated)]
    start_with_cors(
        prepare_app_data,
        configure_app,
        app_port,
        run_env,
        default_cors,
    ).await
}

#[deprecated]
pub async fn start_with_cors<D, T, F, C>
(
    prepare_app_data: impl Fn() -> T + Sized,
    configure_app: F,
    app_port: &str,
    run_env: &str,
    cors_factory: C,
) -> io::Result<()>
where
    D: AppDataWrapper + 'static,
    T: StatsPresenter<D> + 'static + Clone + Send + Sync,
    F: Fn(&mut web::ServiceConfig) + Send + Clone + 'static + Sized,
    C: Fn() -> Cors + Send + Clone + 'static,
{
    Microservice::default()
        .set_app_port(app_port)
        .set_run_env(run_env)
        .start(prepare_app_data, configure_app, Some(cors_factory)).await
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
