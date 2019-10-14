#![warn(
    clippy::all,
    //clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]
#![allow(clippy::single_match_else)]

use ::actix_rt;
use ::actix_web::{
    http, web, App, HttpServer,
    middleware::Logger,
};
use ::actix_cors::Cors;
use ::dotenv::dotenv;
use ::env_logger;

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

pub fn start<T: 'static + Clone + Send, F>
(
    name: &str,
    prepare_app_data: impl Fn() -> T,
    configure_app: F,
    app_port: &str,
    custom_allowed_methods: Option<Vec<&'static str>>,
)
where F: Fn(&mut web::ServiceConfig) + Send + Clone + Copy + 'static
{
    dotenv().ok();
    //env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    let numthreads = threads::num_threads();
    info!("Configuring for {} threads", numthreads);

    info!("Creating actix event loop");
    let sys = actix_rt::System::new(name);

    let app_data = prepare_app_data();

    info!("Starting HTTP server");
    HttpServer::new(move ||
        App::new()
            .data(app_data.clone())
            .configure(configure_app)
            .wrap(Logger::default())
            .wrap(
                Cors::new()
                    .send_wildcard()
                    .allowed_methods(custom_allowed_methods.clone().unwrap_or_else(|| vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]))
                    .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                    .allowed_header(http::header::CONTENT_TYPE)
                    .max_age(3600)
            )
    )
    .workers(numthreads)
    .bind(format!("0.0.0.0:{}", app_port))
    .expect("Can't bind")
    .start();

    info!("Activating actix event loop");
    let _ = sys.run();

}
