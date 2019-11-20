#![warn(
    clippy::all,
    //clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]
#![allow(clippy::single_match_else)]

use ::actix_cors::Cors;
use actix_http::{body::Body, Request, Error};
use ::actix_rt;
use ::actix_service::Service;
use ::actix_web::{
    App, http, HttpServer, test,
    middleware::Logger,
    dev::ServiceResponse,
};
use ::dotenv::dotenv;
use ::env_logger;

#[cfg(feature = "swagger")]
use paperclip::actix::{web, OpenApiExt};
#[cfg(not(feature = "swagger"))]
use actix_web::web;

use super::threads;
#[cfg(feature = "pgsql")]
use super::db_pool;

#[derive(Clone)]
pub struct DefaultAppData {
    #[cfg(feature = "pgsql")]
    pub db_pool: db_pool::Pool,
}

#[cfg(feature = "pgsql")]
pub fn default_app_data() -> DefaultAppData {
    // FIXME: Create Pool as Actix Actor
    info!("Connecting to database");
    let db_pool = db_pool::init_default_pool().unwrap();

    DefaultAppData { db_pool }
}

#[cfg(not(feature = "pgsql"))]
pub fn default_app_data() -> DefaultAppData {
    DefaultAppData { }
}

pub fn start<T, F, C>
(
    name: &str,
    prepare_app_data: impl Fn() -> T,
    configure_app: F,
    app_port: &str,
    cors_factory: Option<C>,
)
where
    T: 'static + Clone + Send,
    F: Fn(&mut web::ServiceConfig) + Send + Clone + Copy + 'static,
    C: Fn() -> Cors + Send + Clone + 'static
{
    dotenv().ok();
    //env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    let numthreads = threads::num_threads();
    info!("Configuring for {} threads", numthreads);

    info!("Creating actix event loop");
    let sys = actix_rt::System::new(name);

    let app_data = prepare_app_data();

    let default_cors_factory = || -> Cors {
        Cors::new()
            .send_wildcard()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
            .max_age(3600)
    };

    info!("Starting HTTP server");
    HttpServer::new(move || {
        let app = App::new();

        #[cfg(feature = "swagger")]
        let app = app.wrap_api()
            .with_json_spec_at("/spec");

        let app = app
            .data(app_data.clone())
            .configure(configure_app)
            .wrap(Logger::default())
            .wrap(
                if let Some(ref cors_factory) = cors_factory {
                    cors_factory()
                } else {
                    default_cors_factory()
                }
            );

            #[cfg(feature = "swagger")]
            let app = app.build();

            app
    })
    .workers(numthreads)
    .bind(format!("0.0.0.0:{}", app_port))
    .expect("Can't bind")
    .start();

    info!("Activating actix event loop");
    let _ = sys.run();

}

pub fn test_init<T, F>(prepare_app_data: impl Fn() -> T, configure_app: F) -> impl Service<Request = Request, Response = ServiceResponse<Body>, Error = Error>
where
    T: 'static,
    F: Fn(&mut web::ServiceConfig),
{
    let app_data = test::run_on(|| prepare_app_data());

    test::init_service({
        let app = App::new();

        #[cfg(feature = "swagger")]
        let app = app.wrap_api();

        let app = app
            .data(app_data)
            .configure(configure_app);

        #[cfg(feature = "swagger")]
        let app = app.build();

        app
    })
}
