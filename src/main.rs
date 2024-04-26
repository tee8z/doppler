use clap::{arg, command, Parser};
use doppler::{create_db, get_absolute_path, run_workflow_until_stop, AppSubCommands, Options};
use fern::colors::{Color, ColoredLevelConfig};
use log::{debug, info, LevelFilter};
use std::{env, fs, io::Error, path::PathBuf};
use time::{format_description::well_known::Iso8601, OffsetDateTime};

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
    setup_logger(&cli).map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let doppler_file_path = get_doppler_file_path(&cli)?;
    debug!("reading doppler file: {}", doppler_file_path);
    let contents = fs::read_to_string(doppler_file_path).expect("file read error");
    debug!("doppler.db location: {}", cli.storage_path);
    let conn = create_db(cli.storage_path).expect("failed to create doppler.db file");
    info!("rest {}", cli.rest);
    let mut options = Options::new(
        cli.docker_dash,
        cli.ui_config_path,
        cli.app_sub_commands,
        conn,
        cli.rest,
        cli.external_nodes,
        cli.network,
    );
    run_workflow_until_stop(&mut options, contents)?;
    info!("successfully cleaned up processes, shutting down");
    Ok(())
}

fn setup_logger(cli: &Cli) -> Result<(), fern::InitError> {
    let rust_log = get_log_level(cli);
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

fn get_log_level(cli: &Cli) -> LevelFilter {
    if cli.level.is_some() {
        let level = cli.level.as_ref().unwrap();
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

pub fn get_doppler_file_path(cli: &Cli) -> Result<String, Error> {
    let file_path = cli.file.to_string_lossy();
    let full_path = get_absolute_path(&file_path).unwrap();
    Ok(full_path.to_string_lossy().to_string())
}
