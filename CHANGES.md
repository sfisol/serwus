<!-- markdownlint-configure-file { "no-duplicate-heading": { "siblings_only": true } } -->

<!-- markdownlint-disable-next-line first-line-h1 -->
## Unreleased

### Added

* `rabbit_tls` feature - `rabbit` with TLS (rustls) for `amqps://` URIs.

### Changed

* The `rabbit` feature uses [`lapin`](https://crates.io/crates/lapin) instead of
  `amiquip`, which is no longer maintained and pinned `idna` 0.1.5 with a known vulnerability
  (RUSTSEC-2024-0421) through `amq-protocol` 1 and `url` 1. `spawn_rabbit_consumer` and
  `send_and_wait_for_ack` take a `lapin::Channel`; `send_and_wait_for_ack` is now `async`,
  returns `SendError::Nack` when the broker refuses the message, and `SendError` implements
  `Debug`, `Display` and `Error`. Errors while declaring queues are logged instead of panicking
  in a blocking task. `crossbeam-channel` is no longer a dependency of the feature. TLS is
  no longer included, enable `rabbit_tls` for it.
* `spawn_rabbit_consumer` hands messages over one at a time and acknowledges each only after
  the actor has handled it, instead of right after queueing it in the actor's mailbox. A
  message that is not valid JSON is rejected without requeueing (dropped, or dead-lettered if
  the queue has a dead letter exchange) instead of staying unacknowledged until the channel
  closes. When the actor has stopped, the consumer cancels itself and requeues what it has not
  handed over.
* `send_and_wait_for_ack` puts the channel into confirm mode only on the first call.

## 0.2.6 - 2026-10-26

### Added

* `set_bind_address` to `Serwus` builder.
* `serwus-test` crate.

## 0.2.5 - 2026-09-21

### Added

* `rs256_jwks_native_roots` feature - same as `rs256_jwks`, but the JWKS client trusts
  the operating system certificate store instead of the bundled `webpki-roots` set.

### Changed

* Updated dependencies (dotenv -> dotenvy, jsonwebtoken to 11)
* `rs256_jwks`: `awc` upgraded from `rustls` 0.20 to `rustls` 0.23, now with the
  `webpki-roots` trust anchors. The JWKS client no longer trusts the operating system
  certificate store, so an authority served with a certificate from a private or
  corporate CA will fail TLS validation with an `UnknownIssuer` error. If that affects
  you, enable `rs256_jwks_native_roots` *instead of* `rs256_jwks`:

  ```toml
  serwus = { version = "0.2", features = ["rs256_jwks_native_roots"] }
  ```

  Enabling both keeps `webpki-roots`, because cargo features are additive and `awc`
  checks `webpki-roots` first. Building now also requires a C toolchain, because
  `rustls` 0.23 uses `aws-lc-rs` as its default crypto provider.

## 0.2.4 - 2026-08-10

### Added

* Granular `r2d2` configuration options to `MultiPoolBuilder`
* `MultiPool::state()` which sums numbers from `r2d2::Pool::state()`

### Performance

* Optimize logger by caching environment variables and ANSI configuration using LazyLock

## 0.2.3 - 2026-01-23

### Changed

* Updated dependencies (diesel 2.3, jsonwebtoken to 10)
* JsonError deserializable
* Refactored pipeline

## 0.2.2 - 2025-10-25

### Added

* More `ErrorBuilder` constructors: `not_found`, `bad_request`, `unauthorized`, `forbidden`

### Changed

* Replaced `quick-error` with `thiserror`

## 0.2.1 - 2025-02-04

### Changed

* Tracing feature now prints special handlers in lowered debug log level (/metrics, /_healthcheck, etc.)
* Updated paperclip to 0.9

## 0.2.0 - 2024-04-22

### Added

* Stable rust compatibility (Remove need for `result_flattening`)
* Prometheus metrics via `metrics-rs`
* Mysql support

### Changed

* Add debug info to errors in default error handler
* Use bunyan format for tracing
* Obfuscate passwords in logs

## 0.1.2 - 2024-01-09

### Fixed

* Reinstated rabbit and actix_validation feature to Cargo.toml

## 0.1.1 - 2023-12-21

### Added

* `SanitizedString`
* More docs

## 0.1.0 - 2023-09-26

Initial release

* MultiPool - Master/replica-aware wrapper for r2d2
* StatsPresenter - Framework for readiness and statistics reporting
* JsonError - Middleware that makes actix-web return errors as JSONs
