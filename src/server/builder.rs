use actix_cors::Cors;
use actix_web::{
    App, HttpServer,
    middleware::{Logger, ErrorHandlers}
};
use dotenv::dotenv;
use log::{info, error};

#[cfg(not(feature = "swagger"))]
use actix_web::web;

#[cfg(feature = "swagger")]
use paperclip::{
    actix::{web, OpenApiExt},
    v2::models::DefaultApiRaw,
};

use crate::server::{default_cors, generic_error::default_error_handler};

use super::threads;

use super::{
    stats::{BaseStats, StatsWrapper, StatsPresenter, AppDataWrapper, default_healthcheck_handler, default_readiness_handler, default_stats_handler},
    logger,
};

pub struct Microservice<'a> {
    app_port: &'a str,
    run_env: &'a str,
    #[cfg(feature = "swagger")]
    swagger_mount: &'a str,
    #[cfg(feature = "swagger")]
    swagger_spec: DefaultApiRaw,
}

impl Default for Microservice<'_>
{
    fn default() -> Self {
        Microservice {
            app_port: "8000",
            run_env: "dev",
            #[cfg(feature = "swagger")]
            swagger_mount: "/swagger",
            #[cfg(feature = "swagger")]
            swagger_spec: DefaultApiRaw::default(),
        }
    }
}

impl<'a> Microservice<'a>
{
    pub fn set_app_port(mut self, app_port: &'a str) -> Self
    {
        self.app_port = app_port;
        self
    }

    pub fn set_run_env(mut self, run_env: &'a str) -> Self
    {
        self.run_env = run_env;
        self
    }

    #[cfg(feature = "swagger")]
    pub fn set_swagger_mount(mut self, swagger_mount: &'a str) -> Self
    {
        self.swagger_mount = swagger_mount;
        self
    }

    #[cfg(feature = "swagger")]
    pub fn set_swagger_spec(mut self, swagger_spec: DefaultApiRaw) -> Self
    {
        self.swagger_spec = swagger_spec;
        self
    }

    #[cfg(feature = "swagger")]
    pub fn set_swagger_info(mut self, pkg_name: impl Into<String>, pkg_version: impl Into<String>, pkg_description: impl Into<String>) -> Self
    {
        self.swagger_spec.info.title = pkg_name.into();
        self.swagger_spec.info.version = pkg_version.into();
        self.swagger_spec.info.description = Some(pkg_description.into());
        self
    }

    pub async fn start<D, T, F, C> (
        self,
        prepare_app_data: impl Fn() -> T + Sized,
        configure_app: F,
        cors_factory: Option<C>,
    ) -> std::io::Result<()>
    where
        D: AppDataWrapper + 'static,
        T: StatsPresenter<D> + 'static + Clone + Send + Sync,
        F: Fn(&mut web::ServiceConfig) + Send + Clone + 'static + Sized,
        C: Fn() -> Cors + Send + Clone + 'static,
    {
        if let Some(cors_factory) = cors_factory {
            self.start_inner(prepare_app_data, configure_app, cors_factory).await
        } else {
            self.start_inner(prepare_app_data, configure_app, default_cors).await
        }
    }

    async fn start_inner<D, T, F, C> (
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

        match logger::init_logger() {
            Ok(_) => info!("Logger has been initialized"),
            Err(_) => error!("Error logger initialization")
        };

        let numthreads = threads::num_threads();
        info!("Configuring for {} threads", numthreads);

        let app_data = web::Data::new(prepare_app_data());
        let stats = web::Data::new(BaseStats::default());

        #[allow(unused)]
        let prod_env = self.run_env == "prod";

        #[cfg(feature = "swagger")]
        let swagger_mount = self.swagger_mount.to_string();

        info!("Starting HTTP server on port {}", self.app_port);
        #[allow(clippy::let_and_return)]
        HttpServer::new(move || {
            let app = App::new()
                .app_data(app_data.clone())
                .app_data(stats.clone())
                .route("_healthcheck", actix_web::web::get().to(default_healthcheck_handler))
                .route("_ready", actix_web::web::get().to(default_readiness_handler::<T, D>))
                .route("_stats", actix_web::web::get().to(default_stats_handler::<T, D>));

            #[cfg(feature = "prometheus")]
            let app = app
                .route("_prometheus", actix_web::web::get().to(super::prometheus::prometheus_stats_handler::<T, D>));

            #[cfg(feature = "swagger")]
            let app = if prod_env {
                app.wrap_api()
            } else {
                app.wrap_api_with_spec(self.swagger_spec.clone())
                    .with_json_spec_at(&format!("{swagger_mount}_spec"))
                    .with_swagger_ui_at(&swagger_mount)
            };

            let app = app
                .configure(configure_app.clone());

            let app = app
                .wrap(cors_factory())
                .wrap(Logger::default())
                .wrap(StatsWrapper::default())
                .wrap(
                    ErrorHandlers::new()
                        .default_handler(default_error_handler)
                );

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
