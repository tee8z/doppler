use crate::{
    copy_file, get_absolute_path, run_command, L1Node, L2Node, NodeCommand, Options, NETWORK,
};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{DependsOnOptions, EnvFile, Networks, Ports, Service, Volumes};
use slog::{debug, error, info};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    str::from_utf8,
    thread,
    time::Duration,
};

const LND_IMAGE: &str = "polarlightning/lnd:0.16.2-beta";

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
    pub bitcoind_node_container_name: String,
}

impl Lnd {
    pub fn get_admin_macaroon(&self) -> Option<String> {
        get_admin_macaroon(self).ok()
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
        get_peers_channel_point(self, options, node_command)
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
        create_lnd_address(self, options)
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

pub fn build_lnd(options: &mut Options, name: &str, pair_name: &str) -> Result<()> {
    let mut lnd_conf = build_and_save_config(options, name, pair_name).unwrap();
    debug!(
        options.global_logger(),
        "{} volume: {}", name, lnd_conf.path_vol
    );

    let rest_port = options.new_port();
    let grpc_port = options.new_port();
    let bitcoind = vec![lnd_conf.bitcoind_node_container_name.clone()];
    let lnd = Service {
        depends_on: DependsOnOptions::Simple(bitcoind),
        image: Some(LND_IMAGE.to_string()),
        container_name: Some(lnd_conf.container_name.clone()),
        ports: Ports::Short(vec![
            format!("{}:{}", options.new_port(), lnd_conf.p2p_port.clone()),
            format!("{}:{}", options.new_port(), lnd_conf.grpc_port.clone()),
            format!("{}:{}", options.new_port(), lnd_conf.rest_port.clone()),
        ]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        volumes: Volumes::Simple(vec![format!("{}:/home/lnd/.lnd:rw", lnd_conf.path_vol)]),
        networks: Networks::Simple(vec![NETWORK.to_owned()]),
        ..Default::default()
    };
    options
        .services
        .insert(lnd_conf.container_name.clone(), Some(lnd));
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

fn build_and_save_config(options: &Options, name: &str, pair_name: &str) -> Result<Lnd, Error> {
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
        .find(|&bitcoind| bitcoind.get_name().eq_ignore_ascii_case(pair_name));
    if let Some(node) = found_node {
        bitcoind_node = node;
    }

    set_l1_values(&mut conf, bitcoind_node)?;
    set_application_options_values(&mut conf, name)?;

    let _ = copy_file(&conf, &destination_dir.clone(), "lnd.conf")?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();
    let container_name = format!("doppler-lnd-{}", name);
    Ok(Lnd {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.to_owned(),
        pubkey: None,
        rpc_server: format!("{}:10000", container_name),
        server_url: format!("http://{}:10000", container_name),
        certificate_path: format!("{}/tls.cert", full_path),
        macaroon_path: format!(
            "{}/data/chain/bitcoin/{}/admin.macaroon",
            full_path, "regtest"
        ),
        path_vol: full_path,
        grpc_port: "10000".to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_node.container_name.clone(),
    })
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
            load_config(
                lnd_name,
                container_name.to_owned(),
                bitcoind_service.to_owned(),
            )
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

fn add_pubkey(node: &mut Lnd, options: &Options) {
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

fn load_config(name: &str, container_name: String, bitcoind_service: String) -> Result<Lnd, Error> {
    let full_path = &format!("data/{}/.lnd", name);
    Ok(Lnd {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.clone(),
        pubkey: None,
        rpc_server: format!("{}:10000", container_name),
        server_url: format!("https://{}:8080", container_name),
        certificate_path: format!("{}/tls.crt", full_path),
        macaroon_path: format!(
            "{}/data/chain/bitcoin/{}/admin.macaroon",
            full_path, "regtest"
        ),
        path_vol: full_path.to_owned(),
        grpc_port: "10000".to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
        bitcoind_node_container_name: bitcoind_service,
    })
}

fn set_l1_values(conf: &mut FileConf, bitcoind_node: &dyn L1Node) -> Result<(), Error> {
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

fn set_application_options_values(conf: &mut FileConf, name: &str) -> Result<(), Error> {
    if conf.sections.get("Application Options").is_none() {
        conf.sections
            .insert("Application Options".to_owned(), Section::new());
    }
    let application_options = conf.sections.get_mut("Application Options").unwrap();
    application_options.set_property("alias", name);
    application_options.set_property("tlsextradomain", name);
    application_options.set_property("restlisten", &format!("0.0.0.0:8080"));
    application_options.set_property("rpclisten", &format!("0.0.0.0:10000"));
    Ok(())
}

fn get_node_pubkey(lnd: &Lnd, options: &Options) -> Result<String, Error> {
    let rpc_command = lnd.get_rpc_server_command();
    let macaroon_path = lnd.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &lnd.container_name,
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "getinfo",
    ];

    let mut retries = 3;
    let mut output_found = None;
    while retries > 0 {
        let output = run_command(options, "pubkey".to_owned(), commands.clone())?;
        if from_utf8(&output.stderr)?.contains(
            "the RPC server is in the process of starting up, but not yet ready to accept calls",
        ) {
            debug!(options.global_logger(), "trying to get pubkey again");
            thread::sleep(Duration::from_secs(2));
            retries -= 1;
        } else {
            output_found = Some(output);
            break;
        }
    }
    if let Some(output) = output_found {
        if output.status.success() {
            if let Some(pubkey) = lnd.get_property("identity_pubkey", output) {
                return Ok(pubkey);
            } else {
                error!(options.global_logger(), "no pubkey found");
            }
        }
    }
    Ok("".to_owned())
}

fn create_lnd_address(lnd: &Lnd, options: &Options) -> Result<String, Error> {
    let rpc_command = lnd.get_rpc_server_command();
    let macaroon_path = lnd.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &lnd.container_name,
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "newaddress",
        "p2tr", // TODO: set as a taproot address by default, make this configurable
    ];
    let output = run_command(options, "newaddress".to_owned(), commands)?;
    let found_address: Option<String> = lnd.get_property("address", output);
    if found_address.is_none() {
        error!(options.global_logger(), "no addess found");
        return Ok("".to_string());
    }
    Ok(found_address.unwrap())
}

fn open_channel(node: &Lnd, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let _ = node.connect(options, node_command).map_err(|e| {
        debug!(options.global_logger(), "failed to connect: {}", e);
    });
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let amt = node_command.amt.unwrap_or(100000).to_string();
    let rpc_command = node.get_rpc_server_command();
    let macaroon_path = node.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();
    let to_pubkey = to_node.get_cached_pubkey();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        node.get_container_name(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "openchannel",
        &to_pubkey,
        &amt,
    ];
    let output = run_command(options, "openchannel".to_owned(), commands)?;
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

fn connect(node: &Lnd, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    info!(
        options.global_logger(),
        "to_node {} {} {} ",
        to_node.get_cached_pubkey(),
        to_node.get_p2p_port(),
        to_node.get_container_name()
    );
    let connection_url = to_node.get_connection_url();
    let rpc_command = node.get_rpc_server_command();
    let macaroon_path = node.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        node.get_container_name(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "connect",
        connection_url.as_ref(),
    ];
    let output = run_command(options, "connect".to_owned(), commands)?;

    if output.status.success() || from_utf8(&output.stderr)?.contains("already connected to peer") {
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

fn close_channel(node: &Lnd, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    //TODO: find a way to specify which channel to close, right now we just grab a random one for this peer
    let peer_channel_point = node.get_peers_channel_point(options, node_command)?;
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let rpc_command = node.get_rpc_server_command();
    let macaroon_path = node.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        node.get_container_name(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "closechannel",
        "--chan_point",
        peer_channel_point.as_ref(),
    ];
    let output = run_command(options, "closechannel".to_owned(), commands)?;

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

fn get_peers_channel_point(
    node: &Lnd,
    options: &Options,
    node_command: &NodeCommand,
) -> Result<String, Error> {
    let to_node = options.get_l2_by_name(node_command.to.as_str())?;
    let rpc_command = node.get_rpc_server_command();
    let macaroon_path = node.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();
    let to_pubkey = to_node.get_cached_pubkey();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        node.get_container_name(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "listchannels",
        "--peer",
        &to_pubkey,
    ];
    let output = run_command(options, "listchannels".to_owned(), commands)?;
    let channel_point = node.get_array_property("channels", "channel_point", output);
    if channel_point.is_none() {
        return Err(anyhow!("no channel point found!"));
    }
    Ok(channel_point.unwrap())
}

fn create_invoice(
    node: &Lnd,
    options: &Options,
    node_command: &NodeCommand,
) -> Result<String, Error> {
    let amt = node_command.amt.unwrap_or(1000).to_string();
    let memo = node.generate_memo();
    let rpc_command = node.get_rpc_server_command();
    let macaroon_path = node.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        node.get_container_name(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "addinvoice",
        "--memo",
        memo.as_ref(),
        "--amt",
        amt.as_ref(),
    ];
    let output = run_command(options, "addinvoice".to_owned(), commands)?;
    let found_payment_request: Option<String> = node.get_property("payment_request", output);
    if found_payment_request.is_none() {
        error!(options.global_logger(), "no payment request found");
    }
    Ok(found_payment_request.unwrap())
}

fn pay_invoice(
    node: &Lnd,
    options: &Options,
    node_command: &NodeCommand,
    payment_request: String,
) -> Result<(), Error> {
    let rpc_command = node.get_rpc_server_command();
    let macaroon_path = node.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        node.get_container_name(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "payinvoice",
        "-f",
        &payment_request,
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
    node: &Lnd,
    options: &Options,
    node_command: &NodeCommand,
    address: &str,
) -> Result<String, Error> {
    let amt = node_command.amt.unwrap_or(1000).to_string();
    let subcommand = node_command.subcommand.to_owned().unwrap_or("".to_owned());
    let rpc_command = node.get_rpc_server_command();
    let macaroon_path = node.get_macaroon_path();
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        node.get_container_name(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        macaroon_path,
        &rpc_command,
        "sendcoins",
        &subcommand,
        "--addr",
        address,
        "--amt",
        &amt,
    ];
    let output = run_command(options, "sendcoins".to_owned(), commands)?;
    let found_tx_id: Option<String> = node.get_property("txid", output);
    if found_tx_id.is_none() {
        error!(options.global_logger(), "no tx id found");
        return Ok("".to_owned());
    }

    Ok(found_tx_id.unwrap())
}

fn get_admin_macaroon(node: &Lnd) -> Result<String, Error> {
    let macaroon_path: String = node.macaroon_path.clone();
    let mut file = OpenOptions::new().read(true).open(macaroon_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let mac_as_hex = hex::encode(buffer);
    Ok(mac_as_hex)
}
