use actix_web::HttpRequest;
use lazy_static::lazy_static;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use paperclip::actix::api_v2_operation;

lazy_static! {
    pub static ref PROM_HANDLER: PrometheusHandle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install recorder");
}

pub async fn metrics(_: HttpRequest) -> String {
    PROM_HANDLER.render()
}
