use anyhow::{anyhow, Error};
use clap::{Args, Subcommand, ValueEnum};
use conf_parser::processer::FileConf;
use docker_compose_types::{Compose, ComposeNetworks, Ipam, MapOrEmpty, Service, Services};
use indexmap::map::IndexMap;
use ipnetwork::IpNetwork;
use slog::{debug, error, Logger};
use std::{
    cell::RefCell,
    fs::{create_dir_all, OpenOptions},
    io::{self, ErrorKind, Read, Write},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::Thread,
    vec,
};

use crate::{
    add_bitcoinds, add_coreln_nodes, add_eclair_nodes, add_lnd_nodes,
    generate_ipv4_sequence_in_subnet, Bitcoind, Cln, Eclair, L1Node, L2Node, Lnd, NETWORK, SUBNET,
};

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

//TODO: make lnd_node and eclair nodes private
#[derive(Clone)]
pub struct Options {
    pub bitcoinds: Vec<Bitcoind>,
    pub lnd_nodes: Vec<Lnd>,
    pub eclair_nodes: Vec<Eclair>,
    pub cln_nodes: Vec<Cln>,
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
            if let Some(AppSubCommands::DetailedCommand(sub_commands)) = app_sub_commands {
                (true, sub_commands.shell_type)
            } else {
                (false, Some(ShellType::default()))
            }
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
            lnd_nodes: vec::Vec::new(),
            eclair_nodes: vec::Vec::new(),
            cln_nodes: vec::Vec::new(),
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
        let target_file = std::path::Path::new(&compose_path);
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
    pub fn get_l2_by_name(&self, name: &str) -> Result<Box<dyn L2Node>, Error> {
        let lnd = self.lnd_nodes.iter().find(|node| node.get_name() == name);
        if lnd.is_none() {
            let eclair_node = self
                .eclair_nodes
                .iter()
                .find(|node| node.get_name() == name);
            if eclair_node.is_none() {
                let core_node = self.cln_nodes.iter().find(|node| node.get_name() == name);
                if core_node.is_none() {
                    return Err(anyhow!("node not found"));
                }
                return Ok(Box::new(core_node.unwrap().to_owned()));
            }
            return Ok(Box::new(eclair_node.unwrap().to_owned()));
        }
        Ok(Box::new(lnd.unwrap().to_owned()))
    }
    pub fn get_l2_nodes(&self) -> Vec<Box<dyn L2Node>> {
        let mut l2_nodes: Vec<Box<dyn L2Node>> = Vec::new();

        for lnd in self.lnd_nodes.iter() {
            l2_nodes.push(Box::new(lnd.clone()));
        }

        for eclair in self.eclair_nodes.iter() {
            l2_nodes.push(Box::new(eclair.clone()));
        }

        for coreln in self.cln_nodes.iter() {
            l2_nodes.push(Box::new(coreln.clone()));
        }

        l2_nodes
    }
    pub fn get_l2_nodes_mut(&self) -> Vec<Rc<RefCell<dyn L2Node>>> {
        let mut l2_nodes: Vec<Rc<RefCell<dyn L2Node>>> = Vec::new();

        for lnd in self.lnd_nodes.iter() {
            l2_nodes.push(Rc::new(RefCell::new(lnd.clone())));
        }

        for eclair in self.eclair_nodes.iter() {
            l2_nodes.push(Rc::new(RefCell::new(eclair.clone())));
        }

        for coreln in self.cln_nodes.iter() {
            l2_nodes.push(Rc::new(RefCell::new(coreln.clone())));
        }

        l2_nodes
    }
    pub fn get_bitcoind_by_name(&self, name: &str) -> Result<&Bitcoind, Error> {
        let btcd = self
            .bitcoinds
            .iter()
            .find(|node| node.get_name() == *name)
            .unwrap_or_else(|| panic!("invalid node name: {:?}", name));
        Ok(btcd)
    }
    pub fn load_bitcoinds(&mut self) -> Result<(), Error> {
        add_bitcoinds(self)?;
        Ok(())
    }
    pub fn load_lnds(&mut self) -> Result<(), Error> {
        add_lnd_nodes(self)
    }
    pub fn load_eclairs(&mut self) -> Result<(), Error> {
        add_eclair_nodes(self)
    }
    pub fn load_coreln(&mut self) -> Result<(), Error> {
        add_coreln_nodes(self)
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
        return get_absolute_path(&destination_file);
    }

    create_dir_all(destination_directory)?;
    conf_parser::processer::write_to_file(source_conf, &destination_file)?;

    get_absolute_path(&destination_file)
}

pub fn create_folder(destination_directory: &str) -> Result<(), anyhow::Error> {
    if Path::new(destination_directory).exists() {
        return Ok(());
    }
    create_dir_all(destination_directory)?;
    Ok(())
}
