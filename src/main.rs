use doppler::{run_workflow_until_stop, DopplerParser, Options, Rule};
use pest::Parser;
use slog::{info, o, Drain, Level, Logger};
use std::{
    env, fs,
    io::{Error, ErrorKind},
};

fn main() -> Result<(), Error> {
    let logger = setup_logger();
    let doppler_file_path = get_doppler_file_path()?;
    let contents = fs::read_to_string(doppler_file_path).expect("file read error");
    let parsed = DopplerParser::parse(Rule::page, &contents)
        .expect("parse error")
        .next()
        .unwrap();

    let options = Options::new(logger.clone());
    run_workflow_until_stop(options, parsed)?;
    info!(logger, "successfully cleaned up processes, shutting down");
    Ok(())
}

fn setup_logger() -> Logger {
    let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| String::from(""));
    let log_level = match rust_log.to_lowercase().as_str() {
        "trace" => Level::Trace,
        "debug" => Level::Debug,
        "info" => Level::Info,
        "warn" => Level::Warning,
        "error" => Level::Error,
        _ => Level::Info,
    };
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = drain.filter_level(log_level).fuse();
    let log = slog::Logger::root(drain, o!("version" => "0.5"));
    info!(log, "log_level {}", log_level);
    log
}

fn get_doppler_file_path() -> Result<String, Error> {
    // Retrieve command-line arguments
    let args: Vec<String> = env::args().collect();

    // Ensure at least one argument is provided (executable name)
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        return Err(Error::new(
            ErrorKind::Other,
            "required a file path to the .doppler file to parse",
        ));
    }

    // Retrieve the value of the file path argument
    Ok(args[1].to_string())
}
