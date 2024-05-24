use std::env;

use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use time::{format_description::well_known::Iso8601, OffsetDateTime};



pub fn setup_logger(log_level: Option<String>) -> Result<(), fern::InitError> {
    let rust_log = get_log_level(log_level);
    let colors = ColoredLevelConfig::new()
        .trace(Color::White)
        .debug(Color::Cyan)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Magenta);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                OffsetDateTime::now_utc().format(&Iso8601::DEFAULT).unwrap(),
                colors.color(record.level()),
                message
            ));
        })
        .level(LevelFilter::Error)
        .level_for("doppler", rust_log)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn get_log_level(log_level: Option<String>) -> LevelFilter {
    if log_level.is_some() {
        let level = log_level.as_ref().unwrap();
        match level.as_ref() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        }
    } else {
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| String::from(""));
        match rust_log.to_lowercase().as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        }
    }
}