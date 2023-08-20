use anyhow::Error;
use conf_parser::processer::FileConf;
use docker_compose_types::{Compose, ComposeNetworks, Service, Services};
use indexmap::map::IndexMap;
use slog::{error, info, Logger};
use std::{
    fs::{create_dir_all, File},
    io::{self, ErrorKind, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::Thread,
    vec,
};

use crate::{MinerTime, NETWORK};

#[derive(Debug, Clone)]
pub struct Options {
    pub bitcoinds: Vec<Bitcoind>,
    pub lnds: Vec<Lnd>,
    ports: Vec<i64>,
    pub compose_path: Option<String>,
    pub services: IndexMap<String, Option<Service>>,
    pub main_thread_active: ThreadController,
    pub main_thread_paused: ThreadController,
    pub loop_stack: IndexMap<String, String>,
    global_logger: Logger,
    thread_handlers: Arc<Mutex<Vec<Thread>>>,
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
    pub fn new(logger: Logger) -> Self {
        let starting_port = vec![9089];
        Self {
            bitcoinds: vec::Vec::new(),
            lnds: vec::Vec::new(),
            ports: starting_port,
            compose_path: None,
            services: indexmap::IndexMap::new(),
            main_thread_active: ThreadController::new(true),
            main_thread_paused: ThreadController::new(false),
            loop_stack: indexmap::IndexMap::new(),
            global_logger: logger,
            thread_handlers: Arc::new(Mutex::new(Vec::new())),
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
    pub fn save_compose(&mut self, file_path: &str) -> Result<(), io::Error> {
        let target_file = std::path::Path::new(file_path);
        let mut networks = IndexMap::new();
        networks.insert(NETWORK.to_owned(), docker_compose_types::MapOrEmpty::Empty);
        let compose = Compose {
            version: Some("3.8".to_string()),
            services: Services(self.services.clone()),
            networks: ComposeNetworks(networks),
            ..Default::default()
        };
        let serialized = match serde_yaml::to_string(&compose) {
            Ok(s) => s,
            Err(e) => panic!("Failed to serialize docker-compose file: {}", e),
        };
        std::fs::write(target_file, serialized).unwrap();
        Ok(())
    }

    pub fn load_compose(&mut self) -> Result<(), io::Error> {
        let compose_path = self.compose_path.clone().unwrap();
        let target_file = std::path::Path::new(compose_path.as_str());
        let mut file = File::open(target_file).map_err(|e| {
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
    pub container_name: Option<String>,
    pub name: Option<String>,
    pub rpchost: String,
    pub rpcport: String,
    pub user: String,
    pub password: String,
    pub zmqpubrawblock: String,
    pub zmqpubrawtx: String,
    pub path_vol: String,
    pub miner_time: Option<MinerTime>,
}

#[derive(Default, Debug, Clone)]
pub struct Lnd {
    pub container_name: Option<String>,
    pub name: Option<String>,
    pub pubkey: Option<String>,
    pub alias: String,
    pub rest_port: String,
    pub grpc_port: String,
    pub p2p_port: String,
    pub server_url: Option<String>,
    pub macaroon_path: Option<String>,
    pub certificate_path: Option<String>,
    pub path_vol: String,
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
