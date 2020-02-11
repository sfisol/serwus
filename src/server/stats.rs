//! Request counter and other stats middleware

use std::collections::{HashSet, HashMap};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::rc::Rc;

use actix_service::{Service, Transform};
use futures::future::{ok as fut_ok, FutureResult};
use futures::{Async, Future, Poll};
use log::debug;

use actix_web::dev::MessageBody;
use actix_web::error::Error;
use actix_web::http::StatusCode;
use actix_web::{
    web::{self, HttpResponse},
    dev::{ServiceRequest, ServiceResponse}
};

use serde::Serialize;

/// BaseStats contains BaseStatsInner singleton
#[derive(Clone)]
pub struct BaseStats(Arc<RwLock<BaseStatsInner>>);

/// BetsStatsInner are common microservice statistics not tied to any special functionality
#[derive(Clone, Serialize)]
pub struct BaseStatsInner {
    request_started: usize,
    request_finished: usize,
    status_codes: HashMap<u16, usize>,
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
        Self::new(excludes)
    }
}

impl<S, B> Transform<S> for StatsWrapper
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = StatsMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

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

impl<S, B> Service for StatsMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = StatsResponse<S, B>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let count_it = !self.config.excludes.contains(req.path());

        if count_it {
            if let Some(stats_arc) = req.app_data::<BaseStats>() {
                if let Ok(mut stats) = stats_arc.0.write() {
                    stats.request_started += 1;
                }
            }
        }

        StatsResponse {
            fut: self.service.call(req),
            count_it,
            _t: PhantomData,
        }
    }
}

#[doc(hidden)]
pub struct StatsResponse<S, B>
where
    B: MessageBody,
    S: Service,
{
    fut: S::Future,
    count_it: bool,
    _t: PhantomData<(B,)>,
}

impl<S, B> Future for StatsResponse<S, B>
where
    B: MessageBody,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Item = ServiceResponse<B>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let res = futures::try_ready!(self.fut.poll());

        if let Some(error) = res.response().error() {
            if res.response().head().status != StatusCode::INTERNAL_SERVER_ERROR {
                debug!("Error in response: {:?}", error);
            }
        }

        if self.count_it {
            let req = res.request();
            if let Some(stats_arc) = req.app_data::<BaseStats>() {
                if let Ok(mut stats) = stats_arc.0.write() {
                    stats.request_finished += 1;
                    let left = stats.request_started - stats.request_finished;
                    if left > 0 {
                        println!("Active clients: {}", left);
                    }
                    let status_code = res.status().as_u16();
                    *stats.status_codes.entry(status_code).or_insert(0) += 1;
                }
            }
        }

        Ok(Async::Ready(res))
    }
}

/// Default alive healthcheck handler
pub fn default_healthcheck_handler() { }

/// Default readiness handler
pub fn default_readiness_handler<S, D>(service_data: web::Data<S>) -> impl Future<Item = HttpResponse, Error = Error>
where
    D: Serialize,
    S: StatsPresenter<D>,
{
    let fut_res = service_data.is_ready()
        .then(|result|
            match result {
                Err(error) => HttpResponse::InternalServerError().body(format!("Can't check readiness: {}", error)),
                Ok(true) => HttpResponse::Ok().finish(),
                Ok(false) => HttpResponse::ServiceUnavailable().finish(),
            }
        );
    Box::new(fut_res)
}

// Default stats handler
pub fn default_stats_handler<S, D>(base_data: web::Data<BaseStats>, service_data: web::Data<S>) -> impl Future<Item = HttpResponse, Error = Error>
where
    D: Serialize,
    S: StatsPresenter<D>,
{
    service_data.get_stats()
        .and_then(move |service_stats| {
            if let Ok(base_stats) = base_data.0.read() {

                #[allow(clippy::unit_arg)]
                let output = StatsOutput {
                    base: base_stats.clone(),
                    service: Some(service_stats),
                };

                HttpResponse::Ok().json(output)
            } else {
                HttpResponse::InternalServerError().body("Can't acquire stats (1)".to_string())
            }
        })
}

#[derive(Serialize)]
pub struct StatsOutput<D: Serialize> {
    base: BaseStatsInner,

    #[serde(skip_serializing_if = "Option::is_none")]
    service: Option<D>,
}

pub trait StatsPresenter<D: Serialize> {
    fn is_ready(&self) -> Box<dyn Future<Item = bool, Error = Error>>;
    fn get_stats(&self) -> Box<dyn Future<Item = D, Error = Error>>;
}

// TODO unittests - see logger tests
