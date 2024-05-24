use anyhow::Error;
use docker_compose_types::{Compose, ComposeNetworks, MapOrEmpty, Service, Services};
use doppler_core::{new, CloneableHashMap, ExternalNode, ImageInfo, NodeKind, OptionLoad, Options};
use doppler_parser::get_absolute_path;
use indexmap::map::IndexMap;
use log::{debug, error};
use rusqlite::Connection;
use std::{
    fs::OpenOptions,
    io::{self, ErrorKind, Read, Write},
    sync::{
        atomic::{AtomicBool, AtomicI64},
        Arc, Mutex,
    },
};

use crate::{
    add_bitcoinds, add_coreln_nodes, add_eclair_nodes, add_external_lnd_nodes, add_lnd_nodes,
    get_latest_polar_images, get_polar_images, update_bash_alias_external, NETWORK,
};

#[derive(Clone)]
pub struct Daemon {
    pub aliases: bool,
    pub shell_type: Option<String>,
    pub docker_command: String,
    pub rest: bool,
    pub external_nodes_path: Option<String>,
    pub ui_config_path: String,
    pub compose_path: Option<String>,
    pub services: IndexMap<String, Option<Service>>,
    default_images: CloneableHashMap<NodeKind, ImageInfo>,
    known_polar_images: CloneableHashMap<NodeKind, Vec<ImageInfo>>,
    pub images: Vec<ImageInfo>,
    ports: Vec<i64>,
    pub options: Options,
}

pub struct SubCommands {
    pub aliases: bool,
    pub shell_type: Option<String>,
}

impl Daemon {
    pub fn new(
        docker_dash: bool,
        ui_config_path: String,
        app_sub_commands: Option<SubCommands>,
        connection: Connection,
        mut rest: bool,
        external_nodes_path: Option<String>,
        network: String,
    ) -> Self {
        let starting_port = vec![9089];

        // Only used when running doppler as an executable
        let (aliases, shell_type) = if let Some(sub_commands) = app_sub_commands {
            (sub_commands.aliases, sub_commands.shell_type)
        } else {
            (false, None)
        };

        let docker_command = if docker_dash {
            "docker-compose"
        } else {
            "docker"
        };
        let latest_polar_images = match get_latest_polar_images() {
            Ok(images) => images,
            Err(err) => panic!("error pulling down latest images: {}", err),
        };
        let all_polar_images = match get_polar_images() {
            Ok(images) => images,
            Err(err) => panic!("error pulling down images: {}", err),
        };
        if external_nodes_path.is_some() {
            rest = true;
        }
        Self {
            default_images: latest_polar_images,
            known_polar_images: all_polar_images,
            images: Vec::new(),
            ports: starting_port,
            compose_path: None,
            services: indexmap::IndexMap::new(),
            aliases,
            shell_type,
            docker_command: docker_command.to_owned(),
            options: Options {
                bitcoinds: Vec::new(),
                lnd_nodes: Vec::new(),
                eclair_nodes: Vec::new(),
                cln_nodes: Vec::new(),
                loop_count: Arc::new(AtomicI64::new(0)),
                read_end_of_doppler_script: Arc::new(AtomicBool::new(true)),
                tags: Arc::new(Mutex::new(new(connection))),
                external_nodes: None,
                network,
                rest,
            },
            external_nodes_path: external_nodes_path,
            ui_config_path,
            rest
        }
    }
    pub fn get_image(&self, name: &str) -> Option<ImageInfo> {
        self.images
            .iter()
            .find(|image| image.is_image(name))
            .map(|image| image.clone())
    }
    pub fn get_default_image(&self, node_kind: NodeKind) -> ImageInfo {
        match self.default_images.get(node_kind) {
            Some(image) => image,
            None => panic!("error no default images found!"),
        }
        .clone()
    }
    pub fn new_port(&mut self) -> i64 {
        let last_port = self.ports.last().unwrap();
        let next_port = last_port + 1;
        self.ports.push(next_port);
        next_port
    }
    pub fn is_known_polar_image(&self, kind: NodeKind, name: &str, tag: &str) -> bool {
        match self.known_polar_images.get(kind) {
            Some(images) => images
                .iter()
                .find(|image| image.get_name() == name && image.get_tag() == tag)
                .is_some(),
            None => false,
        }
    }
    pub fn save_compose(
        &mut self,
        docker_command: String,
        file_path: &str,
    ) -> Result<(), io::Error> {
        let full_path =
            get_absolute_path(file_path).map_err(|e| io::Error::new(ErrorKind::NotFound, e))?;
        debug!("path to new compose: {}", full_path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(full_path)
            .map_err(|e| {
                error!("Failed to open {} file.: {}", docker_command, e);
                io::Error::new(ErrorKind::NotFound, e)
            })?;
        let mut networks = IndexMap::new();
        let network = docker_compose_types::NetworkSettings {
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
                error!("Failed to open compose file.: {}", e);
                io::Error::new(ErrorKind::NotFound, e)
            })?;
        let mut file_content = String::new();
        let doppler_content = file
            .read_to_string(&mut file_content)
            .map_err(|e| {
                error!("Failed to read file.: {}", e);
                io::Error::new(ErrorKind::NotFound, e)
            })
            .map(|_| file_content)?;
        let doppler_compose: Compose =
            serde_yaml::from_str::<Compose>(&doppler_content).map_err(|e| {
                error!("Failed to parse compose file.: {}", e);
                io::Error::new(ErrorKind::InvalidData, e)
            })?;
        let Services(inner_index_map) = doppler_compose.services;
        self.services = inner_index_map;
        Ok(())
    }

    pub fn load_external_nodes(&mut self, external_nodes_file: &str) -> Result<(), Error> {
        let target_file = std::path::Path::new(external_nodes_file);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(target_file)
            .map_err(|e| {
                error!("Failed to open external nodes file.: {}", e);
                io::Error::new(ErrorKind::NotFound, e)
            })?;
        let mut external_nodes: Vec<ExternalNode> = vec![];
        let conf = conf_parser::processer::read_to_file_conf_mut(&file)?;
        for node in conf.sections.clone() {
            if node.0 == "*placeholder*" {
                continue;
            }
            external_nodes.push(ExternalNode {
                node_alias: node.0,
                macaroon_path: node.1.get_property("ADMIN_MACAROON_PATH"),
                api_endpoint: node.1.get_property("API_ENDPOINT"),
                tls_cert_path: node.1.get_property("TLS_CERT_PATH"),
                network: node.1.get_property("NETWORK"),
            })
        }
        self.options.external_nodes = Some(external_nodes);
        if self.aliases {
            update_bash_alias_external(&self.options)?;
        }
        Ok(())
    }
}

impl OptionLoad for Daemon {
    fn load_bitcoinds(&mut self) -> Result<(), Error> {
        add_bitcoinds(&mut self.options)?;
        Ok(())
    }
    fn load_lnds(&mut self) -> Result<(), Error> {
        if let Some(_) = self.options.external_nodes.clone() {
            add_external_lnd_nodes(&mut self.options)
        } else {
            add_lnd_nodes(&mut self.options)
        }
    }
    fn load_eclairs(&mut self) -> Result<(), Error> {
        add_eclair_nodes(self)
    }
    fn load_coreln(&mut self) -> Result<(), Error> {
        add_coreln_nodes(&mut self.options)
    }
}