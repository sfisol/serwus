//! Request counter and other stats middleware

use std::collections::{HashSet, HashMap};
use std::sync::{Arc, RwLock, Weak};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use futures::future::{Future, Ready, ok as fut_ok, FutureExt, TryFutureExt};
use log::{debug, warn};

use actix_web::dev::MessageBody;
use actix_web::error::Error;
use actix_web::http::StatusCode;
use actix_web::{
    web, HttpResponse,
    dev::{ServiceRequest, ServiceResponse}
};

use serde::Serialize;

#[cfg(feature = "prometheus")]
pub use super::prometheus::AsPrometheus;

/// BaseStats contains BaseStatsInner singleton
#[derive(Clone)]
pub struct BaseStats(
    pub(super) Arc<RwLock<BaseStatsInner>>
);

/// BetsStatsInner are common microservice statistics not tied to any special functionality
#[derive(Clone, Serialize)]
pub struct BaseStatsInner {
    pub(super) request_started: usize,
    pub(super) request_finished: usize,
    pub(super) status_codes: HashMap<u16, usize>,
}

impl Default for BaseStats {
    fn default() -> Self {
        Self (
            Arc::new(
                RwLock::new(
                    BaseStatsInner {
                        request_started: 0,
                        request_finished: 0,
                        status_codes: HashMap::new(),
                    }
                )
            )
        )
    }
}

/// Wraps Service with StatMiddleware
pub struct StatsWrapper(Rc<StatsConfig>);

/// Web data with list of handlers to be excluded from statistics
struct StatsConfig {
    excludes: HashSet<String>,
}

impl StatsWrapper {
    pub fn new(excludes: HashSet<String>) -> Self {
        Self(
            Rc::new(
                StatsConfig {
                    excludes
                }
            )
        )
    }
}

impl Default for StatsWrapper {
    fn default() -> Self {
        let mut excludes = HashSet::with_capacity(2);
        excludes.insert("/_healthcheck".to_string());
        excludes.insert("/_ready".to_string());
        excludes.insert("/_stats".to_string());
        #[cfg(feature = "prometheus")]
        excludes.insert("/_prometheus".to_string());
        Self::new(excludes)
    }
}

impl<S, B> Transform<S, ServiceRequest> for StatsWrapper
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    // type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = StatsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        fut_ok(StatsMiddleware {
            service,
            config: self.0.clone(),
        })
    }
}

/// StatsMiddleware counts every request and gathers statistics about returned http codes
pub struct StatsMiddleware<S> {
    service: S,
    config: Rc<StatsConfig>,
}

impl<S, B> Service<ServiceRequest> for StatsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    // type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let count_it = !self.config.excludes.contains(req.path());

        // Count request start-of-handling
        let stats_arc_for_request = req.app_data::<BaseStats>();

        if count_it {
            if let Some(stats_arc) = stats_arc_for_request {
                if let Ok(mut stats) = stats_arc.0.write() {
                    stats.request_started += 1;
                }
            }
        }

        // Get stats reference for later to count stop-of-handling
        // It seems in actix 3 app data can be not available after the call so we get a weak Arc to stats
        let stats_arc_for_response = stats_arc_for_request.map(|bs| Arc::downgrade(&bs.0));

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            if let Some(error) = res.response().error() {
                if res.response().head().status != StatusCode::INTERNAL_SERVER_ERROR {
                    debug!("Error in response: {:?}", error);
                }
            }

            if count_it {
                // Try to acquire strong Arc to stats again
                if let Some(stats_arc) = stats_arc_for_response.map(|wbs| Weak::upgrade(&wbs)).flatten() {
                    if let Ok(mut stats) = stats_arc.write() {
                        stats.request_finished += 1;
                        let left = stats.request_started - stats.request_finished;
                        if left > 1 {
                            warn!("Number of unfinished requests: {}", left);
                        }
                        let status_code = res.status().as_u16();
                        *stats.status_codes.entry(status_code).or_insert(0) += 1;
                    }
                }
            }

            Ok(res)
        })
    }
}

/// Default alive healthcheck handler
pub async fn default_healthcheck_handler() -> &'static str { "" }

/// Default readiness handler
pub async fn default_readiness_handler<S, D>(service_data: web::Data<S>) -> Result<HttpResponse, Error>
where
    D: AppDataWrapper,
    S: StatsPresenter<D>,
{
    let fut_res = service_data.is_ready()
        .map(|result|
            match result {
                Err(error) => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body(format!("Can't check readiness: {}", error)),
                Ok(true) => HttpResponse::build(StatusCode::OK).body("OK".to_string()),
                Ok(false) => HttpResponse::build(StatusCode::SERVICE_UNAVAILABLE).body("Not ready yet".to_string()),
            }
        );
    Ok(fut_res.await)
}

// Default stats handler
pub async fn default_stats_handler<S, D>(base_data: web::Data<BaseStats>, service_data: web::Data<S>) -> Result<HttpResponse, Error>
where
    D: AppDataWrapper,
    S: StatsPresenter<D>,
{
    let fut_res = service_data.get_stats()
        .and_then(move |service_stats| {
            if let Ok(base_stats) = base_data.0.read() {

                #[allow(clippy::unit_arg)]
                let output = StatsOutput {
                    base: base_stats.clone(),
                    service: Some(service_stats),
                };

                fut_ok(HttpResponse::build(StatusCode::OK).body(serde_json::to_string(&output).unwrap()))
            } else {
                fut_ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body("Can't acquire stats (1)".to_string()))
            }
        });

    fut_res.await
}

#[derive(Serialize)]
pub struct StatsOutput<D: Serialize> {
    pub(super) base: BaseStatsInner,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) service: Option<D>,
}

pub trait StatsPresenter<D: AppDataWrapper> {
    fn is_ready(&self) -> Pin<Box<dyn Future<Output=Result<bool, Error>>>>;
    fn get_stats(&self) -> Pin<Box<dyn Future<Output=Result<D, Error>>>>;

    #[cfg(feature = "prometheus")]
    fn get_prometheus(&self) -> Pin<Box<dyn Future<Output=Result<Vec<String>, Error>>>> {
        let fut = self.get_stats().map(|stats_res|
            stats_res.map(|stats| stats.as_prometheus())
        );
        Box::pin(fut)
    }
}

#[cfg(feature = "prometheus")]
pub trait AppDataWrapper: Serialize + AsPrometheus + 'static {}
#[cfg(not(feature = "prometheus"))]
pub trait AppDataWrapper: Serialize {}

#[cfg(feature = "prometheus")]
impl<T> AppDataWrapper for T
where
    T: Serialize + AsPrometheus + 'static
{ }

#[cfg(not(feature = "prometheus"))]
impl<T> AppDataWrapper for T
where
    T: Serialize
{ }

// TODO unittests - see logger tests
