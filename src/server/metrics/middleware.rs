use std::{
    future::{ready, Ready},
    time::Instant,
};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;

#[derive(Debug, Clone)]
pub struct Metrics;

impl<S, B> Transform<S, ServiceRequest> for Metrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = MetricsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MetricsMiddleware { service }))
    }
}

pub struct MetricsMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for MetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let path = req.path().to_owned();
        let method = req.method().to_string();

        let fut = self.service.call(req);

        Box::pin(async move {
            let response = fut.await;

            let latency = start.elapsed().as_secs_f64();
            let status = response
                .as_ref()
                .map(|r| r.status())
                .unwrap_or_else(|e| e.as_response_error().status_code())
                .as_u16()
                .to_string();

            let labels = [("method", method), ("path", path), ("status", status)];

            metrics::counter!("http_requests_total", &labels).increment(1);
            metrics::histogram!("http_requests_duration_seconds", &labels).record(latency);

            response
        })
    }
}
