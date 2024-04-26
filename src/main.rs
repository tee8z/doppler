use clap::{arg, command, Parser};
use doppler::{create_db, get_absolute_path, run_workflow_until_stop, watch, AppSubCommands, Options};
use notify::{RecursiveMode, Watcher};
use slog::{debug, info, o, Drain, Level, Logger};
use std::{env, fs, io::Error, option, path::{Path, PathBuf}};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets path to doppler file
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// Path to watch for files being dropped and to run in order based on time they were added
    #[arg(short, long, default_value = "./ui/static/doppler_files")]
    watch_folder: String,

    /// Set the log level
    #[arg(short, long)]
    level: Option<String>,
    /// Set docker compose command with/without '-'
    #[arg(short, long)]
    docker_dash: bool,

    /// Path to doppler.db, stores tags
    #[arg(short, long, default_value = "./doppler.db")]
    storage_path: String,

    /// Set communication with LND to be REST instead of CLI
    #[arg(short, long)]
    rest: bool,

    /// Set network lightning nodes and bitcoind are running on, default to regtest
    #[arg(short, long, default_value = "regtest")]
    network: String,

    /// Path to override file for external LND nodes
    /// Doppler scripts can only use these nodes matching aliases when set
    #[arg(short, long)]
    external_nodes: Option<String>,

    #[command(subcommand)]
    app_sub_commands: Option<AppSubCommands>,

    /// Path to ui config file, used to connect to the nodes via the browser
    #[arg(short, long, default_value = "./ui/config/info.conf")]
    ui_config_path: String,

}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let logger = setup_logger(&cli);
    let conn = create_db(cli.storage_path.clone()).expect("failed to create doppler.db file");
    debug!(logger, "doppler.db location: {}", cli.storage_path);
    info!(logger, "rest {}", cli.rest);
    let mut options = Options::new(
        logger.clone(),
        cli.docker_dash,
        cli.ui_config_path,
        cli.app_sub_commands,
        conn,
        cli.rest,
        cli.external_nodes,
        cli.network,
        cli.file.is_none()
    );
    if let Some(file) = cli.file {
        let doppler_file_path = get_doppler_file_path(&file.to_string_lossy())?;
        debug!(logger, "reading doppler file: {}", doppler_file_path);
        let contents = fs::read_to_string(doppler_file_path).map_err(Error::other)?;
        run_workflow_until_stop(&mut options, contents)?;
    } else {
        watch(&mut options, &cli.watch_folder).map_err(Error::other)?;
    }
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

pub fn get_doppler_file_path(file_path: &str) -> Result<String, Error> {
    let full_path = get_absolute_path(file_path).unwrap();
    Ok(full_path.to_string_lossy().to_string())
}
