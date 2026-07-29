use chrono::*;
use colored::*;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError, info};
use std::io::Write;
use std::sync::LazyLock;

pub struct ConsoleLogger;

pub static LOGGER: ConsoleLogger = ConsoleLogger;

/// Read once at startup.
static LOGGER_LEVEL: LazyLock<String> =
    LazyLock::new(|| ::std::env::var("LOGGER_LEVEL").unwrap_or_else(|_| "info".to_string()));
static RUN_ENV: LazyLock<String> =
    LazyLock::new(|| ::std::env::var("ENV").unwrap_or_else(|_| "dev".to_string()));
static PROJ_PREFIX: LazyLock<String> =
    LazyLock::new(|| ::std::env::var("PROJECT_PREFIX").unwrap_or_default());

/// Whether ANSI colouring is active for this process (false when stdout is not a terminal, e.g.
/// under Kubernetes). Cached so the plain-text fast path below can skip building `ColoredString`s
/// that would render identically anyway.
static COLORIZE: LazyLock<bool> = LazyLock::new(|| control::SHOULD_COLORIZE.should_colorize());

/// Preformated level labels.
static LEVEL_LABELS: LazyLock<[ColoredString; 5]> = LazyLock::new(|| {
    [
        Level::Error.as_str().red(),
        Level::Warn.as_str().bright_magenta(),
        Level::Info.as_str().green(),
        Level::Debug.as_str().yellow(),
        Level::Trace.as_str().yellow(),
    ]
});

fn level_label(level: Level) -> &'static ColoredString {
    let idx = match level {
        Level::Error => 0,
        Level::Warn => 1,
        Level::Info => 2,
        Level::Debug => 3,
        Level::Trace => 4,
    };
    &LEVEL_LABELS[idx]
}

pub fn logger_level() -> String {
    LOGGER_LEVEL.clone()
}

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info || *LOGGER_LEVEL == "debug"
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = level_label(record.level());
        let module = record.module_path().unwrap_or("");
        let prod = *RUN_ENV != "dev";
        let mut out = std::io::stdout().lock();

        // Error/Warn in a deployed environment: include the source location.
        if prod && matches!(record.level(), Level::Error | Level::Warn) {
            let _ = if *COLORIZE {
                writeln!(
                    out,
                    "[{} {}] {}:{} - {}",
                    level,
                    module,
                    record.file().unwrap_or(""),
                    record.line().unwrap_or(0),
                    record.args().to_string().green(),
                )
            } else {
                writeln!(
                    out,
                    "[{} {}] {}:{} - {}",
                    level,
                    module,
                    record.file().unwrap_or(""),
                    record.line().unwrap_or(0),
                    record.args(),
                )
            };
            return;
        }

        // Anything else in a deployed environment. Debug deliberately falls through to the
        // timestamped format below (subject to its module filter), so that raising
        // `LOGGER_LEVEL=debug` on a deployed pod still surfaces debug lines.
        if prod && record.level() != Level::Debug {
            let _ = writeln!(out, "[{} {}] - {}", level, module, record.args());
            return;
        }

        // Local development, and deployed debug: timestamped, with Debug limited to this project's
        // own modules.
        if module.contains(PROJ_PREFIX.as_str())
            || module.contains(env!("CARGO_PKG_NAME"))
            || record.level() != Level::Debug
        {
            // Only the dev format needs a timestamp, so it is built here rather than for every
            // record regardless of whether it ends up being used.
            let now = Utc::now();
            let (_, year) = now.year_ce();
            let date = format_args!(
                "{}-{:02}-{:02} {:02}:{:02}:{:02}",
                year,
                now.month(),
                now.day(),
                now.hour(),
                now.minute(),
                now.second(),
            );

            let _ = if *COLORIZE {
                writeln!(
                    out,
                    "{}{} {} {}{} {}",
                    "[".white(),
                    date,
                    level,
                    module,
                    "]".white(),
                    record.args().to_string().white(),
                )
            } else {
                writeln!(out, "[{} {} {}] {}", date, level, module, record.args())
            };
        }
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

pub fn init_logger() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)?;

    if *LOGGER_LEVEL == "debug" {
        log::set_max_level(LevelFilter::Debug);
    } else {
        log::set_max_level(LevelFilter::Info);
    }

    info!("Logger init...");

    Ok(())
}
