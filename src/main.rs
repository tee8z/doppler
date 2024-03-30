use clap::{arg, command, Parser};
use doppler::{create_db, get_absolute_path, run_workflow_until_stop, AppSubCommands, Options};
use slog::{debug, info, o, Drain, Level, Logger};
use std::{env, fs, io::Error, path::PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets path to doppler file
    #[arg(short, long, value_name = "FILE")]
    file: PathBuf,

    /// Set the log level
    #[arg(short, long)]
    level: Option<String>,
    /// Set docker compose command with/without '-'
    #[arg(short, long)]
    docker_dash: bool,

    // Path to doppler.db, stores tags
    #[arg(short, long, default_value = "./doppler.db")]
    storage_path: String,

    /// Set communication with LND to be REST instead of CLI
    #[arg(short, long)]
    rest: bool,

    #[command(subcommand)]
    app_sub_commands: Option<AppSubCommands>,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let logger = setup_logger(&cli);

    let doppler_file_path = get_doppler_file_path(&cli)?;
    debug!(logger, "reading doppler file: {}", doppler_file_path);
    let contents = fs::read_to_string(doppler_file_path).expect("file read error");
    debug!(logger, "doppler.db location: {}", cli.storage_path);
    let conn = create_db(cli.storage_path).expect("failed to create doppler.db file");
    let mut options = Options::new(
        logger.clone(),
        cli.docker_dash,
        cli.app_sub_commands,
        conn,
        cli.rest,
    );
    run_workflow_until_stop(&mut options, contents)?;
    info!(logger, "successfully cleaned up processes, shutting down");
    Ok(())
}

fn setup_logger(cli: &Cli) -> Logger {
    let log_level = if cli.level.is_some() {
        let level = cli.level.as_ref().unwrap();
        match level.as_ref() {
            "trace" => Level::Trace,
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warning,
            "error" => Level::Error,
            _ => Level::Info,
        }
    } else {
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| String::from(""));
        match rust_log.to_lowercase().as_str() {
            "trace" => Level::Trace,
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warning,
            "error" => Level::Error,
            _ => Level::Info,
        }
    };

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = drain.filter_level(log_level).fuse();
    let log = slog::Logger::root(drain, o!("version" => "0.5"));
    log
}

pub fn get_doppler_file_path(cli: &Cli) -> Result<String, Error> {
    let file_path = cli.file.to_string_lossy();
    let full_path = get_absolute_path(&file_path).unwrap();
    Ok(full_path.to_string_lossy().to_string())
}
