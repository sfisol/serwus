use log::{info, Record, Level, Metadata, LevelFilter, SetLoggerError};
use chrono::*;
use colored::*;

pub static LOGGER: ConsoleLogger = ConsoleLogger;

pub struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
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
                eprintln!("[{} {}] {}:{} - {}",
                    level,
                    record.module_path().unwrap_or_else(|| ""),
                    record.file().unwrap_or_else(|| ""),
                    record.line().unwrap_or_else(|| 0),
                    format!("{}", record.args()).green(),
                )
            } else {
                println!("{}{} {} {}{} {}",
                    "[".to_string().white(),
                    date, level,
                    record.module_path().unwrap_or_else(|| ""),
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

    let env = ::std::env::var("ENV").unwrap_or_else(|_| "dev".to_string());

    if env == "dev" {
        log::set_max_level(LevelFilter::Debug);
    } else {
        log::set_max_level(LevelFilter::Info);
    }

    info!("Logger init...");

    Ok(())
}
