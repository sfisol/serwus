use actix_cors::Cors;
use actix_web::{middleware::ErrorHandlers, App, HttpServer};
use dotenv::dotenv;

#[cfg(not(feature = "swagger"))]
use actix_web::web;

#[cfg(feature = "swagger")]
use paperclip::{
    actix::{web, OpenApiExt},
    v2::models::DefaultApiRaw,
};

use crate::server::json_error::default_error_handler;

use super::threads;

use super::{
    logger,
    stats::{
        default_healthcheck_handler, default_readiness_handler, default_stats_handler,
        AppDataWrapper, BaseStats, StatsPresenter, StatsWrapper,
    },
};

pub struct Serwus<'a> {
    app_port: &'a str,
    run_env: &'a str,
    #[cfg(feature = "swagger")]
    swagger_mount: &'a str,
    #[cfg(feature = "swagger")]
    swagger_spec: DefaultApiRaw,
    json_errors: bool,
}

impl Default for Serwus<'_> {
    fn default() -> Self {
        Serwus {
            app_port: "8000",
            run_env: "dev",
            #[cfg(feature = "swagger")]
            swagger_mount: "/swagger",
            #[cfg(feature = "swagger")]
            swagger_spec: DefaultApiRaw::default(),
            json_errors: false,
        }
    }
}

impl<'a> Serwus<'a> {
    pub fn set_app_port(mut self, app_port: &'a str) -> Self {
        self.app_port = app_port;
        self
    }

    pub fn set_run_env(mut self, run_env: &'a str) -> Self {
        self.run_env = run_env;
        self
    }

    #[cfg(feature = "swagger")]
    pub fn set_swagger_mount(mut self, swagger_mount: &'a str) -> Self {
        self.swagger_mount = swagger_mount;
        self
    }

    #[cfg(feature = "swagger")]
    pub fn set_swagger_spec(mut self, swagger_spec: DefaultApiRaw) -> Self {
        self.swagger_spec = swagger_spec;
        self
    }

    #[cfg(feature = "swagger")]
    pub fn set_swagger_info(
        mut self,
        pkg_name: impl Into<String>,
        pkg_version: impl Into<String>,
        pkg_description: impl Into<String>,
    ) -> Self {
        self.swagger_spec.info.title = pkg_name.into();
        self.swagger_spec.info.version = pkg_version.into();
        self.swagger_spec.info.description = Some(pkg_description.into());
        self
    }

    // Replaces default error handlers with custom one that
    // any non-JSON error wraps into JSON with GenericError schem
    pub fn json_errors(mut self) -> Self {
        self.json_errors = true;
        self
    }

    pub async fn start<D, T, F, C>(
        self,
        prepare_app_data: impl Fn() -> T + Sized,
        configure_app: F,
        cors_factory: C,
    ) -> std::io::Result<()>
    where
        D: AppDataWrapper + 'static,
        T: StatsPresenter<D> + 'static + Clone + Send + Sync,
        F: Fn(&mut web::ServiceConfig) + Send + Clone + 'static + Sized,
        C: Fn() -> Cors + Send + Clone + 'static,
    {
        dotenv().ok();

        #[cfg(feature = "tracing")]
        {
            use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
            use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
            use tracing_subscriber::util::SubscriberInitExt;

            tracing_subscriber::registry()
                .with(tracing_subscriber::EnvFilter::new(logger::logger_level()))
                .with(JsonStorageLayer)
                .with(BunyanFormattingLayer::new("serwus".into(), std::io::stdout))
                .init();
        }

        #[cfg(not(feature = "tracing"))]
        match logger::init_logger() {
            Ok(_) => log::info!("Logger has been initialized"),
            Err(_) => log::error!("Error logger initialization"),
        };

        let numthreads = threads::num_threads();
        log::info!("Configuring for {} threads", numthreads);

        let app_data = web::Data::new(prepare_app_data());
        let stats = web::Data::new(BaseStats::default());

        #[allow(unused)]
        let prod_env = self.run_env == "prod";

        #[cfg(feature = "swagger")]
        let swagger_mount = self.swagger_mount.to_string();

        log::info!("Starting HTTP server on port {}", self.app_port);
        #[allow(clippy::let_and_return)]
        HttpServer::new(move || {
            let app = App::new()
                .app_data(app_data.clone())
                .app_data(stats.clone())
                .route(
                    "_healthcheck",
                    actix_web::web::get().to(default_healthcheck_handler),
                )
                .route(
                    "_ready",
                    actix_web::web::get().to(default_readiness_handler::<T, D>),
                )
                .route(
                    "_stats",
                    actix_web::web::get().to(default_stats_handler::<T, D>),
                );

            #[cfg(feature = "prometheus")]
            let app = app.route(
                "_prometheus",
                actix_web::web::get().to(super::prometheus::prometheus_stats_handler::<T, D>),
            );

            #[cfg(feature = "metrics")]
            let app = app.wrap(super::metrics::middleware::Metrics).route(
                "metrics",
                actix_web::web::get().to(super::metrics::handler::metrics),
            );

            #[cfg(feature = "swagger")]
            let app = if prod_env {
                app.wrap_api()
            } else {
                app.wrap_api_with_spec(self.swagger_spec.clone())
                    .with_json_spec_at(&format!("{swagger_mount}_spec"))
                    .with_swagger_ui_at(&swagger_mount)
            };

            let app = app.configure(configure_app.clone());

            let app = app
                .wrap(cors_factory())
                .wrap(StatsWrapper::default())
                .wrap({
                    let error_handlers = ErrorHandlers::new();

                    if self.json_errors {
                        error_handlers.default_handler(default_error_handler)
                    } else {
                        error_handlers
                    }
                });

            #[cfg(feature = "tracing")]
            let app = app.wrap(tracing_actix_web::TracingLogger::default());

            let app = app.wrap(actix_web::middleware::Logger::default());

            #[cfg(feature = "swagger")]
            let app = app.build();

            app
        })
        .workers(numthreads)
        .bind(format!("0.0.0.0:{}", self.app_port))
        .expect("Can't bind")
        .run()
        .await
    }
}
