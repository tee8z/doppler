use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{
    AdvancedNetworkSettings, AdvancedNetworks, DependsOnOptions, MapOrEmpty, Networks, Ports,
    Service, Volumes,
};
use indexmap::IndexMap;
use serde_json::{from_slice, Value};
use slog::{debug, error, info};
use std::{
    fs::{File, OpenOptions},
    str::from_utf8,
    thread,
    time::Duration,
};

use crate::{
    copy_file, get_absolute_path, restart_service, run_command, L1Node, L2Node, NodeCommand,
    Options, NETWORK,
};

const ECLAIR_IMAGE: &str = "polarlightning/eclair:0.9.0";

#[derive(Default, Debug, Clone)]
pub struct Eclair {
    pub container_name: String,
    pub name: String,
    pub pubkey: Option<String>,
    pub alias: String,
    pub rest_port: String,
    pub p2p_port: String,
    pub server_url: String,
    pub rpc_server: String,
    pub api_password: String,
    pub path_vol: String,
    pub ip: String,
    pub bitcoind_node_container_name: String,
}

impl Eclair {
    pub fn get_peers_channel_id(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        get_peers_channel_id(self, options, node_command)
    }
}

impl L2Node for Eclair {
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
    fn get_pubkey(&self) -> String {
        self.pubkey.clone().unwrap_or("".to_string())
    }
    fn set_pubkey(&mut self, pubkey: String) {
        self.pubkey = if !pubkey.is_empty() {
            Some(pubkey)
        } else {
            None
        }
    }
    fn get_node_info(&self, options: &Options) -> Result<String, Error> {
        get_node_info(self, options)
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
        create_eclair_address(self, options)
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

pub fn build_eclair(options: &mut Options, name: &str, pair_name: &str) -> Result<()> {
    let ip = options.new_ipv4().to_string();
    let mut eclair_conf = build_config(options, name, pair_name, ip.as_str()).unwrap();
    debug!(
        options.global_logger(),
        "{} volume: {}", name, eclair_conf.path_vol
    );

    let rest_port = options.new_port();
    let mut cur_network = IndexMap::new();
    cur_network.insert(
        NETWORK.to_string(),
        MapOrEmpty::Map(AdvancedNetworkSettings {
            ipv4_address: Some(ip),
            ..Default::default()
        }),
    );
    let bitcoind = vec![eclair_conf.bitcoind_node_container_name.clone()];
    let eclair = Service {
        depends_on: DependsOnOptions::Simple(bitcoind),
        image: Some(ECLAIR_IMAGE.to_string()),
        container_name: Some(eclair_conf.container_name.clone()),
        ports: Ports::Short(vec![eclair_conf.p2p_port.clone()]),
        volumes: Volumes::Simple(vec![format!("{}:/home/eclair:rw", eclair_conf.path_vol)]),
        networks: Networks::Advanced(AdvancedNetworks(cur_network)),
        ..Default::default()
    };
    options
        .services
        .insert(eclair_conf.container_name.clone(), Some(eclair));
    info!(
        options.global_logger(),
        "connect to {} via rest using {} and via grpc using {}",
        eclair_conf.container_name,
        eclair_conf.server_url,
        eclair_conf.rpc_server,
    );
    eclair_conf.rest_port = rest_port.to_string();
    options.eclair_nodes.push(eclair_conf);
    Ok(())
}

fn build_config(options: &Options, name: &str, pair_name: &str, ip: &str) -> Result<Eclair, Error> {
    if options.bitcoinds.is_empty() {
        return Err(anyhow!(
            "bitcoind nodes need to be defined before eclair nodes can be setup"
        ));
    }

    let original = get_absolute_path("config/eclair.conf")?;
    let destination_dir = &format!("data/{}", name);
    let source: File = OpenOptions::new().read(true).write(true).open(original)?;

    let mut conf = read_to_file_conf(&source)?;
    let mut bitcoind_node = options
        .bitcoinds
        .first()
        .expect("a layer 1 needs to be configured before using a layer 2 node");
    let found_node = options
        .bitcoinds
        .iter()
        .find(|&bitcoind| bitcoind.get_name().eq_ignore_ascii_case(pair_name));
    if let Some(node) = found_node {
        bitcoind_node = node;
    }
    let api_password = r#""test1234!""#;
    set_l1_values(
        &mut conf,
        name.to_owned(),
        api_password.to_owned(),
        bitcoind_node,
    )?;

    let _ = copy_file(&conf, &destination_dir.clone(), "eclair.conf")?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();

    Ok(Eclair {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: format!("doppler-eclair-{}", name),
        pubkey: None,
        ip: ip.to_owned(),
        rpc_server: format!("{}:10000", ip),
        server_url: format!("http://{}:10000", ip),
        path_vol: full_path,
        api_password: api_password.to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_node.get_container_name(),
    })
}

fn set_l1_values(
    conf: &mut FileConf,
    name: String,
    api_pass: String,
    bitcoind_node: &dyn L1Node,
) -> Result<(), Error> {
    if conf.sections.get("").is_none() {
        conf.sections.insert("".to_owned(), Section::new());
    }
    let base_section = conf.sections.get_mut("").unwrap();
    base_section.set_property("eclair.node-alias", &name);
    base_section.set_property("eclair.server.port", "9735");
    base_section.set_property("eclair.api.port", "8080");
    base_section.set_property("eclair.api.password", &api_pass);
    base_section.set_property(
        "eclair.bitcoind.zmqblock",
        format!(
            r#""tcp://{}:{}""#,
            bitcoind_node.get_ip(),
            &bitcoind_node.get_zmqpubhashblock()
        )
        .as_str(),
    );
    base_section.set_property(
        "eclair.bitcoind.zmqtx",
        format!(
            r#""tcp://{}:{}""#,
            bitcoind_node.get_ip(),
            &bitcoind_node.get_zmqpubrawtx()
        )
        .as_str(),
    );
    base_section.set_property("eclair.bitcoind.rpcuser", &bitcoind_node.get_rpc_username());
    base_section.set_property(
        "eclair.bitcoind.rpcpassword",
        &format!(r#""{}""#, bitcoind_node.get_rpc_password()),
    );
    base_section.set_property("eclair.bitcoind.auth", "\"password\"");
    base_section.set_property(
        "eclair.bitcoind.host",
        &format!(r#""{}""#, bitcoind_node.get_ip()),
    );
    base_section.set_property("eclair.bitcoind.rpcport", &bitcoind_node.get_rpc_port());

    Ok(())
}

pub fn add_eclair_nodes(options: &mut Options) -> Result<(), Error> {
    let mut node_l2: Vec<_> = options
        .services
        .iter()
        .filter(|service| service.0.contains("eclair"))
        .map(|service| {
            let container_name = service.0;
            let lnd_name = container_name.split('-').last().unwrap();
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
                lnd_name,
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
            let result = node.get_node_info(options);
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

    options.eclair_nodes = nodes;

    Ok(())
}

fn load_config(
    name: &str,
    container_name: String,
    ip: String,
    bitcoind_service: String,
) -> Result<Eclair, Error> {
    let full_path = &format!("data/{}", name);
    Ok(Eclair {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.to_owned(),
        pubkey: None,
        ip: ip.clone(),
        rpc_server: format!("{}:10000", ip),
        server_url: format!("https://{}:8080", ip),
        path_vol: full_path.to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        //TODO: pull this value from the config file
        api_password: "test1234!".to_owned(),
        bitcoind_node_container_name: bitcoind_service,
    })
}

fn get_node_info(node: &Eclair, options: &Options) -> Result<String, Error> {
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "getinfo",
    ];
    let mut retries = 3;
    let mut output_found = None;
    while retries > 0 {
        let output = run_command(options, "pubkey".to_owned(), commands.clone())?;
        if from_utf8(&output.stderr)?.contains("is not running container") {
            debug!(
                options.global_logger(),
                "restarting service and trying to get pubkey again"
            );
            restart_service(options, node.container_name.clone())?;
            thread::sleep(Duration::from_secs(4));
            retries -= 1;
        } else if from_utf8(&output.stderr)?
            .contains("Failed to connect to localhost port 8080: Connection refused")
        {
            thread::sleep(Duration::from_secs(4));
            retries -= 1;
        } else {
            output_found = Some(output);
            break;
        }
    }
    if let Some(output) = output_found {
        if output.status.success() {
            if let Some(pubkey) = node.get_property("nodeId", output.clone()) {
                return Ok(pubkey);
            } else {
                error!(options.global_logger(), "no pubkey found");
            }
        }
    }
    Ok("".to_owned())
}

fn create_eclair_address(node: &Eclair, options: &Options) -> Result<String, Error> {
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "getnewaddress",
    ];
    let output = run_command(options, "getnewaddress".to_owned(), commands)?;
    if output.status.success() {
        let address = from_utf8(&output.stdout).unwrap();
        return Ok(address.trim().to_owned());
    }
    Ok(String::from(""))
}
fn connect(node: &Eclair, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let to_lnd = options.get_l2_by_name(node_command.to.as_str())?;
    let connection_url = to_lnd.get_connection_url();
    let compose_path = options.compose_path.as_ref().unwrap();
    let connection = format!(r#"--uri="{}""#, connection_url.as_str());
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "connect",
        &connection,
    ];
    let output = run_command(options, "connect".to_owned(), commands)?;
    if output.status.success() || from_utf8(&output.stderr)?.contains("already connected") {
        info!(
            options.global_logger(),
            "successfully connected from {} to {}",
            node.get_name(),
            to_lnd.get_name()
        );
    } else {
        error!(
            options.global_logger(),
            "failed to connect from {} to {}",
            node.get_name(),
            to_lnd.get_name()
        );
    }
    Ok(())
}
fn close_channel(
    node: &Eclair,
    options: &Options,
    node_command: &NodeCommand,
) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    //TODO: find a way to specify which channel to close, right now we just grab a random one for this peer
    let peer_channel_id = format!(
        "--channelId={}",
        node.get_peers_channel_id(options, node_command)?
    );
    let to_lnd = options.get_l2_by_name(node_command.to.as_str())?;

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "close",
        &peer_channel_id,
    ];
    let output = run_command(options, "close channel".to_owned(), commands)?;
    if output.status.success() {
        info!(
            options.global_logger(),
            "successfully closed channel from {} to {}",
            node.get_name(),
            to_lnd.get_name()
        );
    } else {
        error!(
            options.global_logger(),
            "failed to close channel from {} to {}",
            node.get_name(),
            to_lnd.get_name()
        );
    }
    Ok(())
}
fn get_peers_channel_id(
    node: &Eclair,
    options: &Options,
    node_command: &NodeCommand,
) -> Result<String, Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let to_lnd = options.get_l2_by_name(node_command.to.as_str())?;
    let to_pubkey = format!("--nodeId={}", to_lnd.get_pubkey());
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "channels",
        &to_pubkey,
    ];
    let output = run_command(options, "channels".to_owned(), commands)?;
    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        match response
            .as_array()
            .and_then(|obj| obj.first())
            .and_then(|item| item.get("channelId"))
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            Some(value) => return Ok(value),
            None => return Ok(String::from("")),
        }
    }
    Ok(String::from(""))
}

fn open_channel(node: &Eclair, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let _ = node.connect(options, node_command).map_err(|e| {
        debug!(options.global_logger(), "failed to connect: {}", e);
    });
    let to_lnd = options.get_l2_by_name(node_command.to.as_str())?;
    let amt = node_command.amt.unwrap_or(100000).to_string();
    let compose_path = options.compose_path.as_ref().unwrap();
    let to_pubkey = format!("--nodeId={}", to_lnd.get_pubkey());
    let funding_command = format!("--fundingSatoshis={}", amt);
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "open",
        &to_pubkey,
        &funding_command,
    ];
    let output = run_command(options, "open channel".to_owned(), commands)?;
    if output.status.success() {
        info!(
            options.global_logger(),
            "successfully opened channel from {} to {}",
            node.get_name(),
            to_lnd.get_name()
        );
    } else {
        error!(
            options.global_logger(),
            "failed to open channel from {} to {}",
            node.get_name(),
            to_lnd.get_name()
        );
    }
    Ok(())
}

fn create_invoice(
    node: &Eclair,
    options: &Options,
    node_command: &NodeCommand,
) -> Result<String, Error> {
    let amt = (node_command.amt.unwrap_or(1000) * 1000).to_string();
    let memo = &format!("--description={}", node.generate_memo());
    let amt_command = &format!("--amountMsat={}", amt);

    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "createinvoice",
        memo,
        amt_command,
    ];
    let output = run_command(options, "createinvoice".to_owned(), commands)?;
    let found_payment_request: Option<String> = node.get_property("serialized", output.clone());
    if found_payment_request.is_none() {
        error!(options.global_logger(), "no payment requests found");
    }
    Ok(found_payment_request.unwrap())
}

fn pay_invoice(
    node: &Eclair,
    options: &Options,
    node_command: &NodeCommand,
    payment_request: String,
) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let invoice_command = format!("--invoice={}", payment_request);
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "payinvoice",
        &invoice_command,
    ];
    let output = run_command(options, "payinvoice".to_owned(), commands)?;
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
    node: &Eclair,
    options: &Options,
    node_command: &NodeCommand,
    address: &str,
) -> Result<String, Error> {
    let amt = node_command.amt.unwrap_or(1000).to_string();
    let address_command = &format!("--address={}", address);
    let amount_command = &format!("--amountSatoshis={}", amt);
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "eclair-cli",
        "-p",
        &node.api_password,
        "sendonchain",
        address_command,
        amount_command,
        "--confirmationTarget=1", //TODO: make it configurable to set number of confirmations
    ];
    let output = run_command(options, "sendonchain".to_owned(), commands)?;
    if !output.status.success() {
        error!(options.global_logger(), "failed to pay on chaing tx");
        return Ok("".to_owned());
    }
    let found_tx_id = from_utf8(&output.stdout)?.trim();

    Ok(found_tx_id.to_owned())
}
