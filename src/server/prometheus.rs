use actix_web::{
    error::Error,
    web::{self, HttpResponse},
};
use futures::Future;
use serde::Serialize;

use super::stats::{StatsPresenter, AppDataWrapper, StatsOutput, BaseStatsInner, BaseStats};

// Prometheus stats handler
pub fn prometheus_stats_handler<S, D>(base_data: web::Data<BaseStats>, service_data: web::Data<S>) -> impl Future<Item = HttpResponse, Error = Error>
where
    D: AppDataWrapper,
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

                HttpResponse::Ok().body(output.to_prometheus().join("\n"))
            } else {
                HttpResponse::InternalServerError().body("Can't acquire stats (1)".to_string())
            }
        })
}

pub trait ToPrometheus {
    fn to_prometheus(&self) -> Vec<String>;
}

impl ToPrometheus for BaseStatsInner {
    fn to_prometheus(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(format!("request_started {}", self.request_started));
        out.push(format!("request_finished {}", self.request_finished));
        for (code, value) in &self.status_codes {
            out.push(format!("status_codes{{code={}}} {}", code, value));
        }
        out
    }
}

impl ToPrometheus for BaseStats {
    fn to_prometheus(&self) -> Vec<String> {
        if let Ok(inner) = self.0.read() {
            inner.to_prometheus()
        } else {
            Vec::new()
        }
    }
}

impl<D> ToPrometheus for StatsOutput<D>
where
    D: ToPrometheus + Serialize
{
    fn to_prometheus(&self) -> Vec<String> {
        let mut out = Vec::new();

        let base_stats = self.base.to_prometheus();
        let service_stats = self.service.to_prometheus();

        for stat in base_stats {
            out.push(format!("base_{}", stat));
        }

        for stat in service_stats {
            out.push(format!("service_{}", stat));
        }
        out
    }
}

impl<T> ToPrometheus for Option<T>
where
    T: ToPrometheus
{
    fn to_prometheus(&self) -> Vec<String> {
        if let Some(t) = self {
            t.to_prometheus()
        } else {
            Vec::new()
        }
    }
}

impl ToPrometheus for () {
    fn to_prometheus(&self) -> Vec<String> {
        Vec::new()
    }
}
