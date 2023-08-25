use crate::{
    copy_file, create_folder, get_absolute_path, run_command, L1Node, L2Node, NodeCommand, Options,
    NETWORK,
};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{
    AdvancedNetworkSettings, AdvancedNetworks, Command, DependsOnOptions, EnvFile, MapOrEmpty,
    Networks, Ports, Service, Volumes,
};
use indexmap::IndexMap;
use slog::{debug, error, info};
use std::{
    fmt::Debug,
    fs::{File, OpenOptions},
    str::from_utf8,
    thread,
    time::Duration,
};

const CLN_IMAGE: &str = "elementsproject/lightningd:v23.05.1";

#[derive(Default, Debug, Clone)]
pub struct Cln {
    pub container_name: String,
    pub name: String,
    pub pubkey: Option<String>,
    pub alias: String,
    pub grpc_port: String,
    pub p2p_port: String,
    pub server_url: String,
    pub path_vol: String,
    pub ip: String,
    pub bitcoind_node_container_name: String,
}

impl Cln {
    pub fn get_connection_url(&self) -> String {
        format!(
            "{}@{}:{}",
            self.pubkey.as_ref().unwrap(),
            self.container_name,
            self.p2p_port.clone()
        )
    }
}

impl L2Node for Cln {
    fn get_connection_url(&self) -> String {
        if let Some(pubkey) = self.pubkey.as_ref() {
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
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn get_container_name(&self) -> &str {
        self.container_name.as_str()
    }
    fn get_ip(&self) -> &str {
        self.ip.as_str()
    }
    fn get_cached_pubkey(&self) -> String {
        self.pubkey.clone().unwrap_or("".to_string())
    }
    fn set_pubkey(&mut self, pubkey: String) {
        self.pubkey = if !pubkey.is_empty() {
            Some(pubkey)
        } else {
            None
        }
    }
    fn get_node_pubkey(&self, options: &Options) -> Result<String, Error> {
        get_node_pubkey(self, options)
    }
    fn open_channel(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        open_channel(self, options, node_command)
    }
    fn connect(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        connect(self, options, node_command)
    }
    fn close_channel(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        close_channel(self, options, node_command)
    }
    fn create_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        create_invoice(self, options, node_command)
    }
    fn pay_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        payment_request: String,
    ) -> Result<(), Error> {
        pay_invoice(self, options, node_command, payment_request)
    }
    fn create_on_chain_address(&self, options: &Options) -> Result<String, Error> {
        create_cln_address(self, options)
    }
    fn pay_address(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        address: &str,
    ) -> Result<String, Error> {
        pay_address(self, options, node_command, address)
    }
}

pub fn build_cln(options: &mut Options, name: &str, pair_name: &str) -> Result<()> {
    let ip = options.new_ipv4().to_string();
    info!(options.global_logger(), "ip: {}", ip);
    let cln_conf = build_and_save_config(options, name, pair_name, ip.as_str()).unwrap();
    debug!(
        options.global_logger(),
        "{} volume: {}", name, cln_conf.path_vol
    );

    let mut cur_network = IndexMap::new();
    cur_network.insert(
        NETWORK.to_string(),
        MapOrEmpty::Map(AdvancedNetworkSettings {
            ipv4_address: Some(ip),
            ..Default::default()
        }),
    );

    // Passing these args on the command line is unavoidable due to how the docker image is setup
    let command = Command::Simple("--network=regtest --lightning-dir=/home/clightning".to_string());
    
    let bitcoind = vec![cln_conf.bitcoind_node_container_name.clone()];
    let cln = Service {
        depends_on: DependsOnOptions::Simple(bitcoind),
        image: Some(CLN_IMAGE.to_string()),
        container_name: Some(cln_conf.container_name.clone()),
        ports: Ports::Short(vec![cln_conf.p2p_port.clone()]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        command: Some(command),
        volumes: Volumes::Simple(vec![format!("{}:/home/clightning:rw", cln_conf.path_vol)]),
        networks: Networks::Advanced(AdvancedNetworks(cur_network)),
        ..Default::default()
    };
    options
        .services
        .insert(cln_conf.container_name.clone(), Some(cln));
    info!(
        options.global_logger(),
        "JTODO: FIXUP: connect to {} - {}", cln_conf.container_name, cln_conf.server_url,
    );

    options.cln_nodes.push(cln_conf);
    Ok(())
}

pub fn build_and_save_config(
    options: &mut Options,
    name: &str,
    pair_name: &str,
    ip: &str,
) -> Result<Cln, Error> {
    if options.bitcoinds.is_empty() {
        return Err(anyhow!(
            "bitcoind nodes need to be defined before cln nodes can be setup"
        ));
    }

    let original = get_absolute_path("config/cln.conf")?;
    let destination_dir = &format!("data/{}", name);
    let source: File = OpenOptions::new().read(true).write(true).open(original)?;

    let mut conf = read_to_file_conf(&source)?;
    let mut bitcoind_node = options
        .bitcoinds
        .first()
        .expect("a layer 1 needs to be confirgured before using a layer 2 node");
    let found_node = options
        .bitcoinds
        .iter()
        .find(|&bitcoind| bitcoind.name.eq_ignore_ascii_case(pair_name));
    if let Some(node) = found_node {
        bitcoind_node = node;
    }

    set_values(&mut conf, name.to_owned(), ip, bitcoind_node)?;
    let _ = copy_file(&conf, &destination_dir.clone(), "config")?;

    // Needed so that the data store in the regtest folder have permissions by the current user and not root
    create_folder(&format!("{}/regtest", destination_dir))?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();

    Ok(Cln {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: format!("doppler-cln-{}", name),
        pubkey: None,
        ip: ip.to_owned(),

        server_url: format!("http://{}:10000", ip),
        path_vol: full_path,
        grpc_port: "10000".to_owned(),

        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_node.container_name.clone(),
    })
}

fn set_values(
    conf: &mut FileConf,
    name: String,
    ip: &str,
    bitcoind_node: &dyn L1Node,
) -> Result<(), Error> {
    if conf.sections.get("").is_none() {
        conf.sections.insert("".to_owned(), Section::new());
    }
    let base_section = conf.sections.get_mut("").unwrap();
    base_section.set_property(
        "bitcoin-rpcconnect",
        format!(
            "{}:{}",
            bitcoind_node.get_ip(),
            &bitcoind_node.get_rpc_port()
        )
        .as_str(),
    );
    base_section.set_property("bitcoin-rpcpassword", &bitcoind_node.get_rpc_password());
    base_section.set_property("bitcoin-rpcuser", &bitcoind_node.get_rpc_username());
    base_section.set_property("bitcoin-rpcport", &bitcoind_node.get_rpc_port());
    base_section.set_property("alias", &name);
    base_section.set_property("bind-addr", &format!("{}:9735", ip));
    base_section.set_property("announce-addr", &format!("{}:9735", ip));
    base_section.set_property("grpc-port", "10000");

    Ok(())
}

pub fn add_coreln_nodes(options: &mut Options) -> Result<()> {
    let mut node_l2: Vec<_> = options
        .services
        .iter()
        .filter(|service| service.0.contains("cln"))
        .map(|service| {
            let container_name = service.0;
            let node_name = container_name.split('-').last().unwrap();
            let mut found_ip: Option<_> = None;
            if let Networks::Advanced(AdvancedNetworks(networks)) =
                service.1.as_ref().unwrap().networks.clone()
            {
                if let MapOrEmpty::Map(advance_setting) = networks.first().unwrap().1 {
                    found_ip = advance_setting.ipv4_address.clone();
                }
            }
            let mut bitcoind_service = "".to_owned();
            if let DependsOnOptions::Simple(layer_1_nodes) =
                service.1.as_ref().unwrap().depends_on.clone()
            {
                bitcoind_service = layer_1_nodes[0].clone();
            }
            load_config(
                node_name,
                container_name.to_owned(),
                found_ip.unwrap(),
                bitcoind_service.to_owned(),
            )
        })
        .filter_map(|res| res.ok())
        .collect();
    let logger = options.global_logger();

    let nodes: Vec<_> = node_l2
        .iter_mut()
        .map(|node| {
            let result = node.get_node_pubkey(options);
            match result {
                Ok(pubkey) => {
                    node.set_pubkey(pubkey);
                    info!(logger, "container: {} found", node.get_name());
                    node.clone()
                }
                Err(e) => {
                    error!(logger, "failed to find node: {}", e);
                    node.clone()
                }
            }
        })
        .collect();

    options.cln_nodes = nodes;

    Ok(())
}

fn load_config(
    name: &str,
    container_name: String,
    ip: String,
    bitcoind_service: String,
) -> Result<Cln, Error> {
    let full_path = &format!("data/{}/.cln", name);
    Ok(Cln {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.to_owned(),
        pubkey: None,
        ip: ip.clone(),
        server_url: format!("https://{}:8080", ip),
        path_vol: full_path.to_owned(),
        grpc_port: "10000".to_owned(),
        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_service,
    })
}

fn get_node_pubkey(node: &Cln, options: &Options) -> Result<String, Error> {
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "getinfo",
    ];
    let mut retries = 3;
    let mut output_found = None;
    while retries > 0 {
        let output = run_command(options, "pubkey".to_owned(), commands.clone())?;
        if from_utf8(&output.stderr)?.contains("is not running container") {
            debug!(
                options.global_logger(),
                "sleeping and trying to get pubkey again"
            );
            thread::sleep(Duration::from_secs(4));
            retries -= 1;
        } else {
            output_found = Some(output);
            break;
        }
    }
    if let Some(output) = output_found {
        if output.status.success() {
            if let Some(pubkey) = node.get_property("id", output.clone()) {
                return Ok(pubkey);
            } else {
                error!(options.global_logger(), "no pubkey found");
            }
        }
    }
    Ok("".to_owned())
}

fn create_cln_address(node: &Cln, options: &Options) -> Result<String, Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "newaddr",
        "bech32",
    ];
    let output = run_command(options, "newaddr".to_owned(), commands)?;
    let found_address: Option<String> = node.get_property("bech32", output.clone());
    if found_address.is_none() {
        error!(options.global_logger(), "no addess found");
        return Ok("".to_string());
    }
    Ok(found_address.unwrap())
}

fn open_channel(node: &Cln, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let _ = node.connect(options, node_command).map_err(|e| {
        debug!(options.global_logger(), "failed to connect: {}", e);
    });
    let amt = node_command.amt.unwrap_or(100000).to_string();
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let to_pubkey = to_node.get_cached_pubkey();
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "fundchannel",
        &to_pubkey,
        &amt
    ];
    let output = run_command(options, "newaddr".to_owned(), commands)?;
    if output.status.success() {
        info!(
            options.global_logger(),
            "successfully opened channel from {} to {}",
            node.get_name(),
            to_node.get_name()
        );
    } else {
        error!(
            options.global_logger(),
            "failed to open channel from {} to {}",
            node.get_name(),
            to_node.get_name()
        );
    }
    Ok(())
}

fn connect(node: &Cln, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let to_pubkey = to_node.get_cached_pubkey();
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "connect",
        &to_pubkey,
        to_node.get_ip(),
        to_node.get_p2p_port()
    ];
    let output = run_command(options, "connect".to_owned(), commands)?;
    if output.status.success() || from_utf8(&output.stderr)?.contains("already connected") {
        info!(
            options.global_logger(),
            "successfully connected from {} to {}",
            node.get_name(),
            to_node.get_name()
        );
    } else {
        error!(
            options.global_logger(),
            "failed to connect from {} to {}",
            node.get_name(),
            to_node.get_name()
        );
    }
    Ok(())
}

fn create_invoice(
    node: &Cln,
    options: &Options,
    node_command: &NodeCommand,
) -> Result<String, Error> {
    let amt = (node_command.amt.unwrap_or(1000) * 1000).to_string();
    let memo = node.generate_memo();
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "invoice",
        &memo,
        &amt,
    ];
    let output = run_command(options, "invoice".to_owned(), commands)?;
    let found_payment_request: Option<String> = node.get_property("serialized", output.clone());
    if found_payment_request.is_none() {
        error!(options.global_logger(), "no payment requests found");
    }
    Ok(found_payment_request.unwrap())
}

fn pay_invoice(
    node: &Cln,
    options: &Options,
    node_command: &NodeCommand,
    payment_request: String,
) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "pay",
        &payment_request,
    ];
    let output = run_command(options, "pay".to_owned(), commands)?;
    if !output.status.success() {
        error!(
            options.global_logger(),
            "failed to make payment from {} to {}", node_command.from, node_command.to
        )
    }
    debug!(
        options.global_logger(),
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(())
}

fn pay_address(
    node: &Cln,
    options: &Options,
    node_command: &NodeCommand,
    address: &str,
) -> Result<String, Error> {
    let amt = node_command.amt.unwrap_or(1000).to_string();
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "withdraw",
        &address,
        &amt,
    ];
    let output = run_command(options, "withdraw".to_owned(), commands)?;
    if !output.status.success() {
        error!(options.global_logger(), "failed to pay on chain tx");
        return Ok("".to_owned());
    }
    let found_tx_id = from_utf8(&output.stdout)?.trim();

    Ok(found_tx_id.to_owned())
}

fn close_channel(node: &Cln, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    //TODO: find a way to specify which channel to close, right now we just grab a random one for this peer
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let to_node_peer_id = to_node.get_cached_pubkey();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        &node.get_container_name(),
        "lightning-cli",
        "--lightning-dir=/home/clightning",
        "--network=regtest",
        "close",
        &to_node_peer_id,
    ];
    let output = run_command(options, "close channel".to_owned(), commands)?;
    if output.status.success() {
        info!(
            options.global_logger(),
            "successfully closed channel from {} to {}",
            node.get_name(),
            to_node.get_name()
        );
    } else {
        error!(
            options.global_logger(),
            "failed to close channel from {} to {}",
            node.get_name(),
            to_node.get_name()
        );
    }
    Ok(())
}
