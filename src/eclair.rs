use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{DependsOnOptions, EnvFile, Networks, Ports, Service, Volumes};
use serde_json::{from_slice, Value};
use slog::{debug, error, info};
use std::{
    fs::{File, OpenOptions},
    str::from_utf8,
    thread,
    time::Duration,
    vec,
};

use crate::{
    copy_file, create_folder, get_absolute_path, restart_service, run_command, ImageInfo, L1Node,
    L2Node, NodeCommand, NodePair, Options, NETWORK,
};

#[derive(Default, Debug, Clone)]
pub struct Eclair {
    pub wallet_starting_balance: i64,
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
    fn get_alias(&self) -> &str {
        &self.alias
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
    fn get_cached_pubkey(&self) -> String {
        self.pubkey.clone().unwrap_or("".to_string())
    }
    fn get_starting_wallet_balance(&self) -> i64 {
        self.wallet_starting_balance
    }
    fn add_pubkey(&mut self, option: &Options) {
        add_pubkey(self, option)
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
    fn force_close_channel(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> std::result::Result<(), Error> {
        force_close_channel(self, options, node_command)
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
    fn get_rhash(&self, option: &Options) -> Result<String, Error> {
        get_rhash(self, option)
    }
    fn get_preimage(&self, _option: &Options, _rhash: String) -> Result<String, Error> {
        unimplemented!();
    }
    fn create_hold_invoice(
        &self,
        _option: &Options,
        _node_command: &NodeCommand,
        _rhash: String,
    ) -> Result<String, Error> {
        // Not implemented yet, needs some more research into their api
        unimplemented!();
    }
    fn settle_hold_invoice(&self, _options: &Options, _preimage: String) -> Result<(), Error> {
        // Not implemented yet, needs some more research into their api
        unimplemented!();
    }
}

pub fn build_eclair(
    options: &mut Options,
    name: &str,
    image: &ImageInfo,
    pair: &NodePair,
) -> Result<()> {
    let mut eclair_conf = build_and_save_config(options, name, pair).unwrap();
    debug!(
        options.global_logger(),
        "{} volume: {}", name, eclair_conf.path_vol
    );

    let rest_port = options.new_port();
    let bitcoind = vec![eclair_conf.bitcoind_node_container_name.clone()];
    let eclair = Service {
        depends_on: DependsOnOptions::Simple(bitcoind),
        image: Some(image.get_image()),
        container_name: Some(eclair_conf.container_name.clone()),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        ports: Ports::Short(vec![
            format!("{}:{}", options.new_port(), eclair_conf.p2p_port),
            format!("{}:{}", options.new_port(), eclair_conf.rest_port),
        ]),
        volumes: Volumes::Simple(vec![format!("{}:/home/eclair:rw", eclair_conf.path_vol)]),
        networks: Networks::Simple(vec![NETWORK.to_owned()]),
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

fn build_and_save_config(options: &Options, name: &str, pair: &NodePair) -> Result<Eclair, Error> {
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
        .find(|&bitcoind| bitcoind.get_name().eq_ignore_ascii_case(&pair.name));
    if let Some(node) = found_node {
        bitcoind_node = node;
    }
    let api_password = r#""test1234!""#;
    set_values(
        &mut conf,
        name.to_owned(),
        api_password.to_owned(),
        bitcoind_node,
    )?;

    let _ = copy_file(&conf, &destination_dir.clone(), "eclair.conf")?;

    // Needed so that the data store in the regtest folder have permissions by the current user and not root
    create_folder(&format!("{}/regtest", destination_dir))?;

    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();
    let container_name = format!("doppler-eclair-{}", name);
    Ok(Eclair {
        wallet_starting_balance: pair.wallet_starting_balance,
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.clone(),
        pubkey: None,
        rpc_server: format!("{}:10000", container_name),
        server_url: format!("http://{}:10000", container_name),
        path_vol: full_path,
        api_password: api_password.to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_node.get_container_name(),
    })
}

fn set_values(
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
            bitcoind_node.get_container_name(),
            &bitcoind_node.get_zmqpubhashblock()
        )
        .as_str(),
    );
    base_section.set_property(
        "eclair.bitcoind.zmqtx",
        format!(
            r#""tcp://{}:{}""#,
            bitcoind_node.get_container_name(),
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
        &format!(r#""{}""#, bitcoind_node.get_container_name()),
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
            let node_name = container_name.split('-').last().unwrap();
            let mut bitcoind_service = "".to_owned();
            if let DependsOnOptions::Simple(layer_1_nodes) =
                service.1.as_ref().unwrap().depends_on.clone()
            {
                bitcoind_service = layer_1_nodes[0].clone();
            }
            load_config(node_name, container_name.to_owned(), bitcoind_service)
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

    options.eclair_nodes = nodes;

    Ok(())
}

fn load_config(
    name: &str,
    container_name: String,
    bitcoind_service: String,
) -> Result<Eclair, Error> {
    let full_path = &format!("data/{}", name);
    Ok(Eclair {
        wallet_starting_balance: 0,
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.to_owned(),
        pubkey: None,
        rpc_server: format!("{}:10000", container_name),
        server_url: format!("https://{}:8080", container_name),
        path_vol: full_path.to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        //TODO: pull this value from the config file
        api_password: "test1234!".to_owned(),
        bitcoind_node_container_name: bitcoind_service,
    })
}

fn add_pubkey(node: &mut Eclair, options: &Options) {
    let result = node.get_node_pubkey(options);
    match result {
        Ok(pubkey) => {
            node.pubkey = Some(pubkey);
            info!(
                options.global_logger(),
                "container: {} found",
                node.get_name()
            );
        }
        Err(e) => {
            error!(options.global_logger(), "failed to find node: {}", e);
        }
    }
}

fn get_node_pubkey(node: &Eclair, options: &Options) -> Result<String, Error> {
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
            if let Some(pubkey) = node.get_property("nodeId", output) {
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
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let connection_url = to_node.get_connection_url();
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
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;

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

fn force_close_channel(
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
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;

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
        "forceclose",
        &peer_channel_id,
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

fn get_peers_channel_id(
    node: &Eclair,
    options: &Options,
    node_command: &NodeCommand,
) -> Result<String, Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let to_pubkey = format!("--nodeId={}", to_node.get_cached_pubkey());
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
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let amt = node_command.amt.unwrap_or(100000).to_string();
    let compose_path = options.compose_path.as_ref().unwrap();
    let to_pubkey = format!("--nodeId={}", to_node.get_cached_pubkey());
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
    let found_payment_request: Option<String> = node.get_property("serialized", output);
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
        error!(options.global_logger(), "failed to pay on chain tx");
        return Ok("".to_owned());
    }
    let found_tx_id = from_utf8(&output.stdout)?.trim();

    Ok(found_tx_id.to_owned())
}

fn get_rhash(node: &Eclair, options: &Options) -> Result<String, Error> {
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
    ];
    let output = run_command(options, "get_rhash".to_owned(), commands)?;
    if !output.status.success() {
        error!(options.global_logger(), "failed to get rhash");
        return Ok("".to_owned());
    }
    let found_rhash = node.get_property("paymentHash", output);
    if found_rhash.is_none() {
        error!(options.global_logger(), "no r_hash found");
        return Ok("".to_owned());
    }
    Ok(found_rhash.unwrap())
}
