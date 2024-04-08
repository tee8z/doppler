use anyhow::{anyhow, Error};
use clap::{Args, Subcommand, ValueEnum};
use conf_parser::processer::FileConf;
use docker_compose_types::{Compose, ComposeNetworks, MapOrEmpty, Service, Services};
use indexmap::map::IndexMap;
use rusqlite::Connection;
use slog::{debug, error, Logger};
use std::{
    fs::{create_dir_all, OpenOptions},
    io::{self, ErrorKind, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc, Mutex,
    },
    thread::Thread,
    vec,
};

use crate::{
    add_bitcoinds, add_coreln_nodes, add_eclair_nodes, add_external_lnd_nodes, add_lnd_nodes,
    get_latest_polar_images, get_polar_images, new, update_bash_alias_external, Bitcoind, Cln,
    CloneableHashMap, Eclair, ImageInfo, L1Node, L2Node, Lnd, NodeKind, Tag, Tags, NETWORK,
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

#[derive(Clone)]
pub struct Options {
    default_images: CloneableHashMap<NodeKind, ImageInfo>,
    known_polar_images: CloneableHashMap<NodeKind, Vec<ImageInfo>>,
    pub images: Vec<ImageInfo>,
    pub bitcoinds: Vec<Bitcoind>,
    pub lnd_nodes: Vec<Lnd>,
    pub eclair_nodes: Vec<Eclair>,
    pub cln_nodes: Vec<Cln>,
    ports: Vec<i64>,
    pub compose_path: Option<String>,
    pub services: IndexMap<String, Option<Service>>,
    pub main_thread_active: ThreadController,
    pub main_thread_paused: ThreadController,
    global_logger: Logger,
    thread_handlers: Arc<Mutex<Vec<Thread>>>,
    pub aliases: bool,
    pub shell_type: Option<ShellType>,
    pub docker_command: String,
    pub loop_count: Arc<AtomicI64>,
    pub read_end_of_doppler_file: Arc<AtomicBool>,
    pub tags: Arc<Mutex<Tags>>,
    pub rest: bool,
    pub external_nodes_path: Option<String>,
    pub external_nodes: Option<Vec<ExternalNode>>,
    pub ui_config_path: String,
}

#[derive(Clone)]
pub struct ExternalNode {
    pub node_alias: String,
    pub macaroon_path: String,
    pub api_endpoint: String,
    pub tls_cert_path: String,
    pub network: String,
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
        ui_config_path: String,
        app_sub_commands: Option<AppSubCommands>,
        connection: Connection,
        mut rest: bool,
        external_nodes_path: Option<String>,
    ) -> Self {
        let starting_port = vec![9089];
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
            images: vec::Vec::new(),
            bitcoinds: vec::Vec::new(),
            lnd_nodes: vec::Vec::new(),
            eclair_nodes: vec::Vec::new(),
            cln_nodes: vec::Vec::new(),
            ports: starting_port,
            compose_path: None,
            services: indexmap::IndexMap::new(),
            main_thread_active: ThreadController::new(true),
            main_thread_paused: ThreadController::new(false),
            global_logger: logger,
            thread_handlers: Arc::new(Mutex::new(Vec::new())),
            aliases,
            shell_type,
            docker_command: docker_command.to_owned(),
            loop_count: Arc::new(AtomicI64::new(0)),
            read_end_of_doppler_file: Arc::new(AtomicBool::new(true)),
            tags: Arc::new(Mutex::new(new(connection))),
            rest: rest,
            external_nodes_path: external_nodes_path,
            external_nodes: None,
            ui_config_path
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

    pub fn load_external_nodes(&mut self, external_nodes_file: &str) -> Result<(), Error> {
        let target_file = std::path::Path::new(external_nodes_file);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(target_file)
            .map_err(|e| {
                error!(
                    self.global_logger(),
                    "Failed to open external nodes file.: {}", e
                );
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
        self.external_nodes = Some(external_nodes);
        if self.aliases {
            update_bash_alias_external(self)?;
        }
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
    pub fn add_pubkeys_l2_nodes(&mut self) -> Result<(), Error> {
        let options_clone = self.clone();
        for lnd in self.lnd_nodes.iter_mut() {
            lnd.add_pubkey(&options_clone);
        }

        for eclair in self.eclair_nodes.iter_mut() {
            eclair.add_pubkey(&options_clone);
        }

        for coreln in self.cln_nodes.iter_mut() {
            coreln.add_pubkey(&options_clone);
        }
        Ok(())
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
        if let Some(_) = self.external_nodes.clone() {
            add_external_lnd_nodes(self)
        } else {
            add_lnd_nodes(self)
        }
    }
    pub fn load_eclairs(&mut self) -> Result<(), Error> {
        add_eclair_nodes(self)
    }
    pub fn load_coreln(&mut self) -> Result<(), Error> {
        add_coreln_nodes(self)
    }
    pub fn save_tag(&self, tag: &Tag) -> Result<(), Error> {
        self.tags
            .lock()
            .unwrap()
            .save(tag.clone())
            .map_err(|e| e.into())
    }
    pub fn get_tag_by_name(&self, name: String) -> Tag {
        self.tags.lock().unwrap().get_by_name(name)
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
