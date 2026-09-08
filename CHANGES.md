<!-- markdownlint-configure-file { "no-duplicate-heading": { "siblings_only": true } } -->

<!-- markdownlint-disable-next-line first-line-h1 -->
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
