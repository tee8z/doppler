use anyhow::Error;
use conf_parser::processer::FileConf;
use docker_compose_types::{Compose, Service, Services, ComposeNetworks};
use indexmap::map::IndexMap;
use std::{
    fs::create_dir_all,
    io,
    path::{Path, PathBuf},
    vec,
};

use crate::NETWORK;

#[derive(Default, Debug)]
pub struct Options {
    pub bitcoinds: Vec<Bitcoind>,
    pub lnds: Vec<Lnd>,
    ports: Vec<i64>,
    pub services: IndexMap<String, Option<Service>>,
}

impl Options {
    pub fn new() -> Self {
        let starting_port = vec![9089];
        Self {
            bitcoinds: vec::Vec::new(),
            lnds: vec::Vec::new(),
            ports: starting_port,
            services: indexmap::IndexMap::new(),
        }
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
        networks.insert(NETWORK.to_owned(),  docker_compose_types::MapOrEmpty::Empty);
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
}

#[derive(Default, Debug)]
pub struct Bitcoind {
    pub container_name: Option<String>,
    pub name: Option<String>,
    pub rpchost: String,
    pub rpcport: String,
    pub user: String,
    pub password: String,
    pub zmqpubrawblock: String,
    pub zmqpubrawtx: String,
    pub path_vol: String,
}

#[derive(Default, Debug)]
pub struct Lnd {
    pub name: Option<String>,
    pub pubkey: Option<String>,
    pub alias: String,
    pub rest_port: String,
    pub grpc_port: String,
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
