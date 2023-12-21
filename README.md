# Serwus

Helpers for building actix-web/diesel based services.

[![crates.io](https://img.shields.io/crates/v/serwus)](https://crates.io/crates/serwus)
[![Documentation](https://docs.rs/serwus/badge.svg)](https://docs.rs/serwus)
![MIT or Apache 2.0 licensed](https://img.shields.io/crates/l/serwus.svg)
[![Dependency Status](https://deps.rs/crate/serwus/0.1.1/status.svg)](https://deps.rs/crate/serwus/0.1.1)
[![CI](https://github.com/sfisol/serwus/actions/workflows/pipeline.yaml/badge.svg)](https://github.com/sfisol/serwus/actions/workflows/pipeline.yaml)
[![downloads](https://img.shields.io/crates/d/serwus.svg)](https://crates.io/crates/serwus)

## Features

* **MultiPool** - Master/replica-aware wrapper for `r2d2`
* **StatsPresenter** - Framework for readiness and statistics reporting
* **JsonError** - Middleware that makes actix-web return errors as JSONs

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
