<!-- markdownlint-configure-file { "no-duplicate-heading": { "siblings_only": true } } -->

<!-- markdownlint-disable-next-line first-line-h1 -->
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
