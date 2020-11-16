use log::{info, Record, Level, Metadata, LevelFilter, SetLoggerError};
use chrono::*;
use colored::*;

pub struct ConsoleLogger;

pub static LOGGER: ConsoleLogger = ConsoleLogger;

pub fn logger_level() -> String {
    ::std::env::var("LOGGER_LEVEL").unwrap_or_else(|_| "info".to_string())
}

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info || logger_level() == "debug"
    }

    fn log(&self, record: &Record) {
        let now = Utc::now();
        let hour = now.hour();
        let (_, year) = now.year_ce();

        let date = format!(
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            year, now.month(), now.day(),
            hour,
            now.minute(),
            now.second(),
        );

        if self.enabled(record.metadata()) {
            let level = if record.level() == Level::Info {
                format!("{}", record.level()).green()
            } else if record.level() == Level::Error {
                format!("{}", record.level()).red()
            } else {
                format!("{}", record.level()).yellow()
            };

            let env = ::std::env::var("ENV").unwrap_or_else(|_| "dev".to_string());

            if [Level::Error, Level::Warn].contains(&record.level()) && env != "dev" {
                println!("[{} {}] {}:{} - {}",
                    level,
                    record.module_path().unwrap_or(""),
                    record.file().unwrap_or(""),
                    record.line().unwrap_or(0),
                    format!("{}", record.args()).green(),
                )
            } else {
                println!("{}{} {} {}{} {}",
                    "[".to_string().white(),
                    date, level,
                    record.module_path().unwrap_or(""),
                    "]".to_string().white(),
                    format!("{}", record.args()).white(),
                )
            }
        }
    }

    fn flush(&self) {}
}

pub fn init_logger() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)?;

    if logger_level() == "debug" {
        log::set_max_level(LevelFilter::Debug);
    } else {
        log::set_max_level(LevelFilter::Info);
    }

    info!("Logger init...");

    Ok(())
}
