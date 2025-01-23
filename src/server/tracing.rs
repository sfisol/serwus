use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::Error;
use tracing::Span;
use tracing_actix_web::{DefaultRootSpanBuilder, Level, RootSpanBuilder};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::logger;

pub struct TracingSpanBuilder;

const HUSHED_PATHS: [&str; 6] = [
    "/_ready",
    "/_healthcheck",
    "/_stats",
    "/_prometheus",
    "/metrics",
    "/swagger",
];

impl RootSpanBuilder for TracingSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        let level = if HUSHED_PATHS.contains(&request.path()) {
            Level::DEBUG
        } else {
            Level::INFO
        };

        tracing_actix_web::root_span!(level = level, request)
    }

    fn on_request_end<B: MessageBody>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        DefaultRootSpanBuilder::on_request_end(span, outcome);
    }
}

pub fn register_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(logger::logger_level()))
        .with(JsonStorageLayer)
        .with(BunyanFormattingLayer::new("serwus".into(), std::io::stdout))
        .init();
}
