# Serwus

Helpers for building actix-web/diesel based services.

[![crates.io](https://img.shields.io/crates/v/serwus)](https://crates.io/crates/serwus)
[![Documentation](https://docs.rs/serwus/badge.svg)](https://docs.rs/serwus)
![MIT or Apache 2.0 licensed](https://img.shields.io/crates/l/serwus.svg)
[![Dependency Status](https://deps.rs/crate/serwus/0.2.8/status.svg)](https://deps.rs/crate/serwus/0.2.8)
[![CI](https://github.com/sfisol/serwus/actions/workflows/pipeline.yaml/badge.svg)](https://github.com/sfisol/serwus/actions/workflows/pipeline.yaml)
[![downloads](https://img.shields.io/crates/d/serwus.svg)](https://crates.io/crates/serwus)

## Helpers

* **MultiPool** (`multidb` feature) - Master/replica-aware wrapper for `r2d2`
* **StatsPresenter** - Framework for readiness and statistics reporting
* **JsonError** - Middleware that makes actix-web return errors as JSONs
* **Serwus::compress** - Compression of all responses of the service (brotli, gzip, zstd)
* **load_env** - Reads `.env` into the environment once, without overriding variables already set
* **Session cookie** (`session` feature) - Opaque session token in an `HttpOnly` cookie, stored as a hash

## Example

```rust
use actix_web::web;
use serwus::{
    server::{Serwus, default_cors},
    EmptyStats
};

#[derive(Clone, EmptyStats)]
pub struct AppData;

async fn hello() -> &'static str {
    "Hello world\n"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let prepare_app_data = || AppData;

    Serwus::default()
        .start(
            prepare_app_data,
            |app| {
                app.route("/", web::get().to(hello));
            },
            default_cors,
        )
        .await
}
```

## Cargo features

| Feature | Default | Description |
| --- | :---: | --- |
| `pgsql` | yes | PostgreSQL through `diesel` with an `r2d2` pool (`db_pool`) |
| `mysql` | | MySQL through `diesel`; `pgsql` wins when both are enabled |
| `multidb` | | `MultiPool` - master/replica pools instead of a single one |
| `auth` | yes | JWT, password hashing and roles, with the RustCrypto backend of `jsonwebtoken` |
| `auth_aws_lc_rs` | | `auth` with the `aws-lc-rs` backend instead |
| `auth_core` | | `auth` without a `jsonwebtoken` backend, for an application that selects it itself |
| `session` | | Sessions in a cookie (`auth::session`) |
| `rs256_jwks` | | Validation of RS256 tokens against a 3rd-party JWKS, trusting `webpki-roots` |
| `rs256_jwks_native_roots` | | `rs256_jwks` trusting the operating system certificate store instead |
| `http2` | yes | HTTP/2 in the actix-web server |
| `tracing` | yes | Request tracing with JSON (bunyan) logs instead of the plain logger |
| `metrics` | | Metrics middleware and a Prometheus `metrics` endpoint |
| `prometheus` | | Service stats in the Prometheus format at `_prometheus` |
| `swagger` | | OpenAPI v2 spec and Swagger UI (`paperclip`), outside the `prod` environment |
| `openapi_v3` | | `swagger` with the OpenAPI v3 spec too |
| `actix_validation` | | `render_single_validation_error` - a `validator` error as an actix-web error |
| `rabbit` | | RabbitMQ through `lapin`: a consumer handing messages to an actor, confirmed publishing |
| `rabbit_tls` | | `rabbit` with TLS for `amqps://` URIs (needs a C toolchain and CMake) |
