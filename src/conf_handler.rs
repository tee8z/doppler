use anyhow::Error;
use clap::{Args, Subcommand, ValueEnum};
use conf_parser::processer::FileConf;
use docker_compose_types::{Compose, ComposeNetworks, Ipam, MapOrEmpty, Service, Services};
use indexmap::map::IndexMap;
use ipnetwork::IpNetwork;
use slog::{debug, error, Logger};
use std::{
    fs::{create_dir_all, OpenOptions},
    io::{self, ErrorKind, Read, Write},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::Thread,
    vec,
};

use crate::{generate_ipv4_sequence_in_subnet, MinerTime, NETWORK, SUBNET};

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

#[derive(Debug, Clone)]
pub struct Options {
    pub bitcoinds: Vec<Bitcoind>,
    pub lnds: Vec<Lnd>,
    ports: Vec<i64>,
    ip_addresses: Vec<Ipv4Addr>,
    pub compose_path: Option<String>,
    pub services: IndexMap<String, Option<Service>>,
    pub main_thread_active: ThreadController,
    pub main_thread_paused: ThreadController,
    pub loop_stack: IndexMap<String, String>,
    global_logger: Logger,
    thread_handlers: Arc<Mutex<Vec<Thread>>>,
    pub aliases: bool,
    pub shell_type: Option<ShellType>,
    pub docker_command: String,
}

#[derive(Default, Debug, Clone)]
pub struct ThreadController {
    active: Arc<AtomicBool>,
}

impl ThreadController {
    fn new(val: bool) -> Self {
        ThreadController {
            active: Arc::new(AtomicBool::new(val)),
        }
    }
    pub fn set(&self, val: bool) {
        self.active.store(val, Ordering::Relaxed);
    }

    pub fn val(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

impl Options {
    pub fn new(
        logger: Logger,
        docker_dash: bool,
        app_sub_commands: Option<AppSubCommands>,
    ) -> Self {
        let starting_port = vec![9089];
        let starting_ip = "10.5.0.2";
        let cidr = IpNetwork::from_str(starting_ip).unwrap();
        let start_ip = match cidr {
            IpNetwork::V4(cidr_v4) => cidr_v4.network(),
            _ => panic!("Only IPv4 is supported"),
        };
        let (aliases, shell_type) = if app_sub_commands.is_some() {
            let AppSubCommands::DetailedCommand(sub_commands) = app_sub_commands.unwrap();
            (true, sub_commands.shell_type)
        } else {
            (true, Some(ShellType::default()))
        };
        let docker_command = if docker_dash {
            "docker-compose"
        } else {
            "docker"
        };
        Self {
            bitcoinds: vec::Vec::new(),
            lnds: vec::Vec::new(),
            ports: starting_port,
            ip_addresses: vec![start_ip],
            compose_path: None,
            services: indexmap::IndexMap::new(),
            main_thread_active: ThreadController::new(true),
            main_thread_paused: ThreadController::new(false),
            loop_stack: indexmap::IndexMap::new(),
            global_logger: logger,
            thread_handlers: Arc::new(Mutex::new(Vec::new())),
            aliases,
            shell_type,
            docker_command: docker_command.to_owned(),
        }
    }
    pub fn global_logger(&self) -> Logger {
        self.global_logger.clone()
    }
    pub fn add_thread(&self, thread_handler: Thread) {
        self.thread_handlers.lock().unwrap().push(thread_handler);
    }
    pub fn get_thread_handlers(&self) -> Arc<Mutex<Vec<Thread>>> {
        self.thread_handlers.clone()
    }
    pub fn get_bitcoind(&self, name: String) -> Bitcoind {
        self.bitcoinds
            .iter()
            .find(|bitcoind| bitcoind.name == name)
            .unwrap()
            .clone()
    }
    pub fn new_port(&mut self) -> i64 {
        let last_port = self.ports.last().unwrap();
        let next_port = last_port + 1;
        self.ports.push(next_port);
        next_port
    }
    pub fn new_ipv4(&mut self) -> Ipv4Addr {
        let last_ip = self.ip_addresses.last().unwrap();
        let next_ip = generate_ipv4_sequence_in_subnet(self.global_logger(), SUBNET, last_ip);
        self.ip_addresses.push(next_ip);
        next_ip
    }
    pub fn save_compose(
        &mut self,
        docker_command: String,
        file_path: &str,
    ) -> Result<(), io::Error> {
        let full_path =
            get_absolute_path(file_path).map_err(|e| io::Error::new(ErrorKind::NotFound, e))?;
        debug!(
            self.global_logger(),
            "path to new compose: {}",
            full_path.display()
        );
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(full_path)
            .map_err(|e| {
                error!(
                    self.global_logger(),
                    "Failed to open {} file.: {}", docker_command, e
                );
                io::Error::new(ErrorKind::NotFound, e)
            })?;
        let mut networks = IndexMap::new();
        let network = docker_compose_types::NetworkSettings {
            driver: Some("bridge".to_owned()),
            ipam: Some(Ipam {
                driver: None,
                config: vec![docker_compose_types::IpamConfig {
                    subnet: SUBNET.to_string(),
                    gateway: Some("10.5.0.1".to_string()),
                }],
            }),
            ..Default::default()
        };
        networks.insert(NETWORK.to_owned(), MapOrEmpty::Map(network));
        let compose = Compose {
            version: Some("3.8".to_string()),
            services: Services(self.services.clone()),
            networks: ComposeNetworks(networks),
            ..Default::default()
        };
        let serialized = match serde_yaml::to_string(&compose) {
            Ok(s) => s,
            Err(e) => panic!("Failed to serialize {} file: {}", docker_command, e),
        };
        file.write_all(serialized.as_bytes()).map_err(|e| {
            io::Error::new(
                ErrorKind::NotFound,
                format!("failed to write new docker compose file: {}", e),
            )
        })?;
        Ok(())
    }

    pub fn load_compose(&mut self) -> Result<(), io::Error> {
        let compose_path = self.compose_path.clone().unwrap();
        let target_file = std::path::Path::new(compose_path.as_str());
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(target_file)
            .map_err(|e| {
                error!(self.global_logger(), "Failed to open compose file.: {}", e);
                io::Error::new(ErrorKind::NotFound, e)
            })?;
        let mut file_content = String::new();
        let doppler_content = file
            .read_to_string(&mut file_content)
            .map_err(|e| {
                error!(self.global_logger(), "Failed to read file.: {}", e);
                io::Error::new(ErrorKind::NotFound, e)
            })
            .map(|_| file_content)?;
        let doppler_compose: Compose =
            serde_yaml::from_str::<Compose>(&doppler_content).map_err(|e| {
                error!(self.global_logger(), "Failed to parse compose file.: {}", e);
                io::Error::new(ErrorKind::InvalidData, e)
            })?;
        let Services(inner_index_map) = doppler_compose.services;
        self.services = inner_index_map;
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct Bitcoind {
    pub conf: FileConf,
    pub data_dir: String,
    pub container_name: String,
    pub name: String,
    pub p2pport: String,
    pub rpcport: String,
    pub user: String,
    pub password: String,
    pub zmqpubrawblock: String,
    pub zmqpubrawtx: String,
    pub path_vol: String,
    pub ip: String,
    pub miner_time: Option<MinerTime>,
}

#[derive(Default, Debug, Clone)]
pub struct Lnd {
    pub container_name: String,
    pub name: String,
    pub pubkey: Option<String>,
    pub alias: String,
    pub rest_port: String,
    pub grpc_port: String,
    pub p2p_port: String,
    pub server_url: String,
    pub rpc_server: String,
    pub macaroon_path: String,
    pub certificate_path: String,
    pub path_vol: String,
    pub ip: String,
}

impl Lnd {
    pub fn get_rpc_server_command(&self) -> String {
        format!("--rpcserver={}:10000", self.ip)
    }
    pub fn get_macaroon_command(&self) -> String {
        "--macaroonpath=/home/lnd/.lnd/data/chain/bitcoin/regtest/admin.macaroon".to_owned()
    }
    pub fn get_connection_url(&self) -> String {
        format!(
            "{}@{}:{}",
            self.pubkey.as_ref().unwrap(),
            self.container_name,
            self.p2p_port.clone()
        )
    }
}

pub fn get_absolute_path(relative_path: &str) -> Result<PathBuf, Error> {
    let current_dir = std::env::current_dir()?;
    let absolute_path = current_dir.join(relative_path);

    Ok(absolute_path)
}

pub fn copy_file(
    source_conf: &FileConf,
    destination_directory: &str,
    conf_name: &str,
) -> Result<PathBuf, anyhow::Error> {
    let destination_file = format!("{}/{}", destination_directory, conf_name);
    if Path::new(destination_directory).exists() {
        //TODO: figure out how to update conf file in directory between runs
        return get_absolute_path(&destination_file);
    }

    create_dir_all(destination_directory)?;
    conf_parser::processer::write_to_file(source_conf, &destination_file)?;

    get_absolute_path(&destination_file)
}
