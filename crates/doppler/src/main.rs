use clap::{arg, command, Parser};
use fern::colors::{Color, ColoredLevelConfig};
use log::{debug, info, LevelFilter};
use std::{env, fs, io::Error, path::PathBuf};
use time::{format_description::well_known::Iso8601, OffsetDateTime};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Sets path to doppler file
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

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

#[derive(Subcommand)]
pub enum AppSubCommands {
    #[command(about = "aliases settings", name = "aliases")]
    DetailedCommand(Script),
}

#[derive(Args, Debug)]
pub struct Script {
    /// Set the shell language to use for the aliases file
    #[arg(value_enum)]
    pub shell_type: Option<ShellType>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ShellType {
    ZSH,
    KSH,
    CSH,
    SH,
    #[default]
    BASH,
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellType::ZSH => write!(f, "#!/bin/zsh"),
            ShellType::KSH => write!(f, "#!/bin/ksh"),
            ShellType::CSH => write!(f, "#!/bin/csh"),
            ShellType::SH => write!(f, "#!/bin/sh"),
            ShellType::BASH => write!(f, "#!/bin/bash"),
        }
    }
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    setup_logger(&cli).map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let conn = create_db(cli.storage_path.clone()).expect("failed to create doppler.db file");
    debug!("doppler.db location: {}", cli.storage_path);
    info!("using rest communication {}", cli.rest);
    let mut options = Options::new(
        cli.docker_dash,
        cli.ui_config_path,
        cli.app_sub_commands,
        conn,
        cli.rest,
        cli.external_nodes,
        cli.network,
    );
    let file = cli.file.expect("doppler file not found");
    let doppler_file_path = get_doppler_file_path(&file.to_string_lossy())?;
    debug!("reading doppler file: {}", doppler_file_path);
    let contents = fs::read_to_string(doppler_file_path).map_err(Error::other)?;
    run_workflow_until_stop(&mut options, contents)?;
    info!("successfully cleaned up processes, shutting down");
    Ok(())
}
