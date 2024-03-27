use crate::{
    add_rest_client, copy_file, get_absolute_path, ImageInfo, L1Node, L2Node, LndCli, LndRest,
    NodeCommand, NodePair, Options, NETWORK,
};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{DependsOnOptions, EnvFile, Networks, Ports, Service, Volumes};
use slog::{debug, error, info};
use std::fs::{File, OpenOptions};

#[derive(Default, Debug, Clone)]
pub struct Lnd {
    pub wallet_starting_balance: i64,
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
    pub bitcoind_node_container_name: String,
    pub lnd_cli: LndCli,
    pub lnd_rest: Option<LndRest>, // If we are using rest, we assume this is a mutinynet node hosted remotely
}

impl Lnd {
    pub fn get_admin_macaroon(&self) -> Option<String> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.get_admin_macaroon(self).ok()
        } else {
            self.lnd_cli.get_admin_macaroon(self).ok()
        }
    }

    pub fn get_macaroon_path(&self) -> &str {
        "--macaroonpath=/home/lnd/.lnd/data/chain/bitcoin/regtest/admin.macaroon"
    }
    pub fn get_rpc_server_command(&self) -> String {
        "--rpcserver=localhost:10000".to_owned()
    }
    /// A channel point is the outpoint (txid:index) of the funding transaction.
    pub fn get_peers_channel_point(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.get_peers_channel_point(options, node_command)
        } else {
            self.lnd_cli
                .get_peers_channel_point(self, options, node_command)
        }
    }
}

impl L2Node for Lnd {
    fn get_connection_url(&self) -> String {
        if let Some(pubkey) = self.pubkey.clone() {
            format!(
                "{}@{}:{}",
                pubkey,
                self.container_name,
                self.p2p_port.clone()
            )
        } else {
            "".to_owned()
        }
    }
    fn get_p2p_port(&self) -> &str {
        self.p2p_port.as_str()
    }
    fn get_server_url(&self) -> &str {
        self.server_url.as_str()
    }
    fn get_alias(&self) -> &str {
        &self.alias
    }
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn get_container_name(&self) -> &str {
        self.container_name.as_str()
    }
    fn get_cached_pubkey(&self) -> String {
        self.pubkey.clone().unwrap_or("".to_string())
    }
    fn get_starting_wallet_balance(&self) -> i64 {
        self.wallet_starting_balance
    }
    fn add_pubkey(&mut self, options: &Options) {
        if options.rest {
            if let Some(lnd_rest) = self.lnd_rest.clone() {
                self.lnd_rest = Some(add_rest_client(lnd_rest).unwrap());
            }
        }
        let result = self.get_node_pubkey(options);
        match result {
            Ok(pubkey) => {
                self.pubkey = Some(pubkey);
                info!(
                    options.global_logger(),
                    "container: {} found",
                    self.get_name()
                );
            }
            Err(e) => {
                error!(options.global_logger(), "failed to find node: {}", e);
            }
        }
    }
    fn get_node_pubkey(&self, options: &Options) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.get_node_pubkey(options)
        } else {
            self.lnd_cli.get_node_pubkey(self, options)
        }
    }
    fn open_channel(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.open_channel(self, options, node_command)
        } else {
            self.lnd_cli.open_channel(self, options, node_command)
        }
    }
    fn connect(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.connect(self, options, node_command)
        } else {
            self.lnd_cli.connect(self, options, node_command)
        }
    }
    fn close_channel(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.close_channel(self, options, node_command)
        } else {
            self.lnd_cli.close_channel(self, options, node_command)
        }
    }
    fn force_close_channel(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> std::result::Result<(), Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.force_close_channel(self, options, node_command)
        } else {
            self.lnd_cli
                .force_close_channel(self, options, node_command)
        }
    }
    fn create_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.create_invoice(self, options, node_command)
        } else {
            self.lnd_cli.create_invoice(self, options, node_command)
        }
    }
    fn pay_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        payment_request: String,
    ) -> Result<(), Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.pay_invoice(options, node_command, payment_request)
        } else {
            self.lnd_cli
                .pay_invoice(self, options, node_command, payment_request)
        }
    }
    fn create_on_chain_address(&self, options: &Options) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.create_lnd_address(options)
        } else {
            self.lnd_cli.create_lnd_address(self, options)
        }
    }
    fn pay_address(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        address: &str,
    ) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.pay_address(options, node_command, address)
        } else {
            self.lnd_cli
                .pay_address(self, options, node_command, address)
        }
    }
    fn get_rhash(&self, options: &Options) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.get_rhash(options)
        } else {
            self.lnd_cli.get_rhash(self, options)
        }
    }
    fn get_preimage(&self, options: &Options, rhash: String) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.get_preimage(options, rhash)
        } else {
            self.lnd_cli.get_preimage(self, options, rhash)
        }
    }
    fn create_hold_invoice(
        &self,
        option: &Options,
        node_command: &NodeCommand,
        rhash: String,
    ) -> Result<String, Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.create_hold_invoice(option, node_command, rhash)
        } else {
            self.lnd_cli
                .create_hold_invoice(self, option, node_command, rhash)
        }
    }
    fn settle_hold_invoice(&self, options: &Options, preimage: String) -> Result<(), Error> {
        if let Some(rest) = self.lnd_rest.clone() {
            rest.settle_hold_invoice(options, &preimage)
        } else {
            self.lnd_cli.settle_hold_invoice(self, options, &preimage)
        }
    }
}

pub fn build_lnd(
    options: &mut Options,
    name: &str,
    image: &ImageInfo,
    pair: &NodePair,
) -> Result<()> {
    let mut lnd_conf = build_and_save_config(options, name, image, pair).unwrap();
    debug!(
        options.global_logger(),
        "{} volume: {}", name, lnd_conf.path_vol
    );

    let rest_port = options.new_port();
    let grpc_port = options.new_port();
    let p2p_port = options.new_port();
    let bitcoind = vec![lnd_conf.bitcoind_node_container_name.clone()];
    let lnd = Service {
        depends_on: DependsOnOptions::Simple(bitcoind),
        image: Some(image.get_image()),
        container_name: Some(lnd_conf.container_name.clone()),
        ports: Ports::Short(vec![
            format!("{}:{}", p2p_port, lnd_conf.p2p_port),
            format!("{}:{}", grpc_port, lnd_conf.grpc_port),
            format!("{}:{}", rest_port, lnd_conf.rest_port),
        ]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        volumes: Volumes::Simple(vec![format!("{}:/home/lnd/.lnd:rw", lnd_conf.path_vol)]),
        networks: Networks::Simple(vec![NETWORK.to_owned()]),
        ..Default::default()
    };

    options
        .services
        .insert(lnd_conf.container_name.clone(), Some(lnd));
    lnd_conf.server_url = format!("https://localhost:{}", rest_port.to_string());
    if options.rest {
        lnd_conf.lnd_rest = Some(LndRest::new(
            &lnd_conf.server_url,
            lnd_conf.macaroon_path.clone(),
            lnd_conf.certificate_path.clone(),
        )?)
    }
    info!(
        options.global_logger(),
        "connect to {} via rest using {} and via grpc using {} with admin.macaroon found at {}",
        lnd_conf.container_name,
        lnd_conf.server_url,
        lnd_conf.rpc_server,
        lnd_conf.macaroon_path.clone(),
    );
    lnd_conf.grpc_port = grpc_port.to_string();
    lnd_conf.rest_port = rest_port.to_string();

    options.lnd_nodes.push(lnd_conf);
    Ok(())
}

fn build_and_save_config(
    options: &Options,
    name: &str,
    _image: &ImageInfo,
    pair: &NodePair,
) -> Result<Lnd, Error> {
    if options.bitcoinds.is_empty() {
        return Err(anyhow!(
            "bitcoind nodes need to be defined before lnd nodes can be setup"
        ));
    }

    let original = get_absolute_path("config/lnd.conf")?;
    let destination_dir = &format!("data/{}/.lnd", name);
    let source: File = OpenOptions::new().read(true).write(true).open(original)?;

    let mut conf = read_to_file_conf(&source)?;
    let mut bitcoind_node = options
        .bitcoinds
        .first()
        .expect("a layer 1 needs to be confirgured before using a layer 2 node");
    let found_node = options
        .bitcoinds
        .iter()
        .find(|&bitcoind| bitcoind.get_name().eq_ignore_ascii_case(&pair.name));
    if let Some(node) = found_node {
        bitcoind_node = node;
    }

    set_l1_values(&mut conf, bitcoind_node)?;
    let container_name = format!("doppler-lnd-{}", name);

    set_application_options_values(&mut conf, name, &container_name)?;

    let _ = copy_file(&conf, &destination_dir.clone(), "lnd.conf")?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();
    let macaroon_path = format!(
        "{}/data/chain/bitcoin/{}/admin.macaroon",
        full_path, "regtest"
    );
    let server_url = format!("http://{}:10000", container_name);
    Ok(Lnd {
        wallet_starting_balance: pair.wallet_starting_balance,
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.to_owned(),
        pubkey: None,
        rpc_server: format!("{}:10000", container_name),
        server_url: server_url.clone(),
        certificate_path: format!("{}/tls.cert", full_path),
        macaroon_path: macaroon_path.clone(),
        path_vol: full_path,
        grpc_port: "10000".to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_node.container_name.clone(),
        lnd_cli: LndCli,
        lnd_rest: None, //we set this later in the process when the forwarding ports are determined
    })
}

pub fn load_config(
    name: &str,
    container_name: String,
    bitcoind_service: String,
    rest_port: Option<&str>,
) -> Result<Lnd, Error> {
    let full_path = &format!("data/{}/.lnd", name);
    let macaroon_path = format!(
        "{}/data/chain/bitcoin/{}/admin.macaroon",
        full_path, "regtest"
    );
    let tls_path = format!("{}/tls.cert", full_path);
    let server_url = if let Some(rest_port) = rest_port {
        format!("https://localhost:{}", rest_port)
    } else {
        format!("http://{}:8080", container_name)
    };
    let lnd_rest = if let Some(_) = rest_port {
        Some(LndRest::new(
            &server_url,
            macaroon_path.clone(),
            tls_path.clone(),
        )?)
    } else {
        None
    };

    Ok(Lnd {
        wallet_starting_balance: 0,
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.clone(),
        pubkey: None,
        rpc_server: format!("{}:10000", container_name),
        server_url: server_url.clone(),
        certificate_path: tls_path,
        macaroon_path: macaroon_path.clone(),
        path_vol: full_path.to_owned(),
        grpc_port: "10000".to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_service,
        lnd_cli: LndCli,
        lnd_rest: lnd_rest,
    })
}

pub fn set_l1_values(conf: &mut FileConf, bitcoind_node: &dyn L1Node) -> Result<(), Error> {
    if conf.sections.get("Bitcoin").is_none() {
        conf.sections.insert("Bitcoin".to_owned(), Section::new());
    }
    let bitcoin = conf.sections.get_mut("Bitcoin").unwrap();
    bitcoin.set_property("bitcoin.active", "true");
    bitcoin.set_property("bitcoin.regtest", "true");
    bitcoin.set_property("bitcoin.node", "bitcoind");

    if conf.sections.get("Bitcoind").is_none() {
        conf.sections.insert("Bitcoind".to_owned(), Section::new());
    }
    let bitcoind = conf.sections.get_mut("Bitcoind").unwrap();
    bitcoind.set_property(
        "bitcoind.zmqpubrawblock",
        format!(
            "tcp://{}:{}",
            bitcoind_node.get_container_name(),
            &bitcoind_node.get_zmqpubrawblock()
        )
        .as_str(),
    );
    bitcoind.set_property(
        "bitcoind.zmqpubrawtx",
        format!(
            "tcp://{}:{}",
            bitcoind_node.get_container_name(),
            &bitcoind_node.get_zmqpubrawtx()
        )
        .as_str(),
    );
    bitcoind.set_property("bitcoind.rpcpass", &bitcoind_node.get_rpc_password());
    bitcoind.set_property("bitcoind.rpcuser", &bitcoind_node.get_rpc_username());
    bitcoind.set_property(
        "bitcoind.rpchost",
        format!(
            "{}:{}",
            bitcoind_node.get_container_name(),
            &bitcoind_node.get_rpc_port()
        )
        .as_str(),
    );

    Ok(())
}

pub fn set_application_options_values(
    conf: &mut FileConf,
    name: &str,
    container_name: &str,
) -> Result<(), Error> {
    if conf.sections.get("Application Options").is_none() {
        conf.sections
            .insert("Application Options".to_owned(), Section::new());
    }
    let application_options = conf.sections.get_mut("Application Options").unwrap();
    application_options.set_property("alias", name);
    application_options.set_property("tlsextradomain", container_name);
    application_options.set_property("restlisten", &String::from("0.0.0.0:8080"));
    application_options.set_property("rpclisten", &String::from("0.0.0.0:10000"));
    Ok(())
}

pub fn add_lnd_nodes(options: &mut Options) -> Result<(), Error> {
    let mut node_l2: Vec<_> = options
        .services
        .iter()
        .filter(|service| service.0.contains("lnd"))
        .map(|service| {
            let container_name = service.0;
            let lnd_name = container_name.split('-').last().unwrap();
            let mut bitcoind_service = "".to_owned();
            if let DependsOnOptions::Simple(layer_1_nodes) =
                service.1.as_ref().unwrap().depends_on.clone()
            {
                bitcoind_service = layer_1_nodes[0].clone();
            }
            if let Ports::Short(ports) = service.1.clone().unwrap().ports {
                let rest_port = ports.iter().find_map(|port| {
                    let split_ports: Vec<&str> = port.split(":").collect();
                    if split_ports[1] == "8080" {
                        Some(split_ports[0])
                    } else {
                        None
                    }
                });
                return load_config(
                    lnd_name,
                    container_name.to_owned(),
                    bitcoind_service,
                    rest_port,
                );
            }
            load_config(lnd_name, container_name.to_owned(), bitcoind_service, None)
        })
        .filter_map(|res| res.ok())
        .collect();

    let nodes: Vec<_> = node_l2
        .iter_mut()
        .map(|node| {
            node.add_pubkey(options);
            node.clone()
        })
        .collect();

    options.lnd_nodes = nodes;

    Ok(())
}
