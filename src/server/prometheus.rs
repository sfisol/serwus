use actix_web::body::BoxBody;
use actix_web::{error::Error, HttpResponse, web};
use actix_web::http::StatusCode;
use futures::future::{ok as fut_ok, TryFutureExt};
use serde::Serialize;

use super::stats::{StatsPresenter, AppDataWrapper, StatsOutput, BaseStatsInner, BaseStats};

// Prometheus stats handler
pub async fn prometheus_stats_handler<S, D>(base_data: web::Data<BaseStats>, service_data: web::Data<S>) -> Result<HttpResponse<BoxBody>, Error>
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

                fut_ok(HttpResponse::build(StatusCode::OK).body(output.as_prometheus().join("\n")))
            } else {
                fut_ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body("Can't acquire stats (1)".to_string()))
            }
        });

    fut_res.await
}

pub trait AsPrometheus {
    fn as_prometheus(&self) -> Vec<String>;
}

impl AsPrometheus for BaseStatsInner {
    fn as_prometheus(&self) -> Vec<String> {
        let mut out = vec![
            format!("request_started {}", self.request_started),
            format!("request_finished {}", self.request_finished),
        ];
        for (code, value) in &self.status_codes {
            out.push(format!("status_codes{{code=\"{code}\"}} {value}"));
        }
        out
    }
}

impl AsPrometheus for BaseStats {
    fn as_prometheus(&self) -> Vec<String> {
        if let Ok(inner) = self.0.read() {
            inner.as_prometheus()
        } else {
            Vec::new()
        }
    }
}

impl<D> AsPrometheus for StatsOutput<D>
where
    D: AsPrometheus + Serialize
{
    fn as_prometheus(&self) -> Vec<String> {
        let mut out = Vec::new();

        let base_stats = self.base.as_prometheus();
        let service_stats = self.service.as_prometheus();

        for stat in base_stats {
            out.push(format!("base_{stat}"));
        }

        for stat in service_stats {
            out.push(format!("service_{stat}"));
        }
        out
    }
}

impl<T> AsPrometheus for Option<T>
where
    T: AsPrometheus
{
    fn as_prometheus(&self) -> Vec<String> {
        if let Some(t) = self {
            t.as_prometheus()
        } else {
            Vec::new()
        }
    }
}

impl AsPrometheus for () {
    fn as_prometheus(&self) -> Vec<String> {
        Vec::new()
    }
}
