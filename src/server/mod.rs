pub mod stats;
pub mod app_data;
#[cfg(feature = "prometheus")]
pub mod prometheus;

use actix_cors::Cors;
use actix_http::{body::Body, Request};
use actix_service::Service;
use actix_web::{
    App, http, HttpServer, test, Error,
    middleware::Logger,
    dev::ServiceResponse,
};
use dotenv::dotenv;
use log::{info, error};
use std::io;

#[cfg(not(feature = "swagger"))]
use actix_web::web;

#[cfg(feature = "swagger")]
use paperclip::actix::{web, OpenApiExt};

use super::threads;

#[cfg(feature = "pgsql")]
use super::db_pool;

use super::logger;
use stats::{BaseStats, StatsWrapper, StatsPresenter, AppDataWrapper, default_healthcheck_handler, default_readiness_handler, default_stats_handler};

pub use app_data::{DefaultAppData, default_app_data};

fn default_cors() -> Cors {
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
        .max_age(3600)
}

pub async fn start<D, T, F>
(
    prepare_app_data: impl Fn() -> T,
    configure_app: F,
    app_port: &str,
    run_env: &str,
) -> io::Result<()>
where
    D: AppDataWrapper + 'static,
    T: StatsPresenter<D> + 'static + Clone + Send,
    F: Fn(&mut web::ServiceConfig) + Send + Clone + Copy + 'static,
{
    start_with_cors(
        prepare_app_data,
        configure_app,
        app_port,
        run_env,
        default_cors,
    ).await
}

pub async fn start_with_cors<D, T, F, C>
(
    prepare_app_data: impl Fn() -> T,
    configure_app: F,
    app_port: &str,
    run_env: &str,
    cors_factory: C,
) -> io::Result<()>
where
    D: AppDataWrapper + 'static,
    T: StatsPresenter<D> + 'static + Clone + Send,
    F: Fn(&mut web::ServiceConfig) + Send + Clone + Copy + 'static,
    C: Fn() -> Cors + Send + Clone + 'static,
{
    dotenv().ok();

    match logger::init_logger() {
        Ok(_) => info!("Logger has been initialized"),
        Err(_) => error!("Error logger initialization")
    };

    //env::set_var("RUST_LOG", "actix_web=debug");

    let numthreads = threads::num_threads();
    info!("Configuring for {} threads", numthreads);

    let app_data = prepare_app_data();

    let stats = BaseStats::default();

    #[allow(unused)]
    let prod_env = run_env == "prod";

    info!("Starting HTTP server");
    #[allow(clippy::let_and_return)]
    HttpServer::new(move || {
        let app = App::new()
            .route("_healthcheck", actix_web::web::get().to(default_healthcheck_handler))
            .route("_ready", actix_web::web::get().to(default_readiness_handler::<T, D>))
            .route("_stats", actix_web::web::get().to(default_stats_handler::<T, D>));

        #[cfg(feature = "prometheus")]
        let app = app
            .route("_prometheus", actix_web::web::get().to(prometheus::prometheus_stats_handler::<T, D>));

        #[cfg(feature = "swagger")]
        let app = if prod_env {
            app.wrap_api()
        } else {
            app.wrap_api()
                .with_json_spec_at("/spec")
        };

        let app = app
            .app_data(web::Data::new(app_data.clone()))

            // Needs to be added in two waysin actix 4, maybe because of: https://github.com/actix/actix-web/issues/1790
            .app_data(web::Data::new(stats.clone()))
            .app_data(stats.clone())

            .configure(configure_app)
            .wrap(cors_factory())
            .wrap(Logger::default())
            .wrap(StatsWrapper::default());

        #[cfg(feature = "swagger")]
        let app = app.build();

        app
    })
        .workers(numthreads)
        .bind(format!("0.0.0.0:{}", app_port))
        .expect("Can't bind")
        .run()
        .await
}

pub async fn test_init<T, F>(prepare_app_data: impl Fn() -> T, configure_app: F) -> impl Service<Request, Response = ServiceResponse<Body>, Error = Error>
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
