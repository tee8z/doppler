use crate::{
    copy_file, get_absolute_path, mine_to_address, run_command, Bitcoind, Lnd, Options, NETWORK,
};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{
    AdvancedNetworkSettings, AdvancedNetworks, EnvFile, MapOrEmpty, Networks, Ports, Service,
    Volumes,
};
use indexmap::IndexMap;
use rand::Rng;
use serde_yaml::{from_slice, Value};
use slog::{debug, error, info, Logger};
use std::{
    fmt::Debug,
    fs::{File, OpenOptions},
    process::Output,
    str::from_utf8,
    thread,
    time::Duration,
};

const LND_IMAGE: &str = "polarlightning/lnd:0.16.2-beta";

pub fn build_lnd(options: &mut Options, name: &str, pair_name: &str) -> Result<()> {
    let ip = options.new_ipv4().to_string();
    let mut lnd_conf = get_lnd_config(options, name, pair_name, ip.as_str()).unwrap();
    debug!(
        options.global_logger(),
        "{} volume: {}", name, lnd_conf.path_vol
    );

    let rest_port = options.new_port();
    let grpc_port = options.new_port();
    let mut cur_network = IndexMap::new();
    cur_network.insert(
        NETWORK.to_string(),
        MapOrEmpty::Map(AdvancedNetworkSettings {
            ipv4_address: Some(ip),
            ..Default::default()
        }),
    );
    let lnd = Service {
        image: Some(LND_IMAGE.to_string()),
        container_name: Some(lnd_conf.container_name.clone()),
        ports: Ports::Short(vec![lnd_conf.p2p_port.clone()]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        volumes: Volumes::Simple(vec![format!("{}:/home/lnd/.lnd:rw", lnd_conf.path_vol)]),
        networks: Networks::Advanced(AdvancedNetworks(cur_network)),
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

    options.lnds.push(lnd_conf);
    Ok(())
}

pub fn get_lnd_config(
    options: &mut Options,
    name: &str,
    pair_name: &str,
    ip: &str,
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
        .find(|&bitcoind| bitcoind.name.eq_ignore_ascii_case(pair_name));
    if let Some(node) = found_node {
        bitcoind_node = node;
    }

    set_bitcoind_values(&mut conf, bitcoind_node)?;
    set_application_options_values(&mut conf, name, ip)?;

    let _ = copy_file(&conf, &destination_dir.clone(), "lnd.conf")?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();

    Ok(Lnd {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: format!("doppler-lnd-{}", name),
        pubkey: None,
        ip: ip.to_owned(),
        rpc_server: format!("{}:10000", ip),
        server_url: format!("http://{}:10000", ip),
        certificate_path: format!("{}/tls.crt", full_path),
        macaroon_path: format!(
            "{}/data/chain/bitcoin/{}/admin.macaroon",
            full_path, "regtest"
        ),
        path_vol: full_path,
        grpc_port: "10000".to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
    })
}

pub fn get_lnds(options: &mut Options) -> Result<()> {
    let mut lnds: Vec<Lnd> = options
        .clone()
        .services
        .into_iter()
        .filter(|service| service.0.contains("lnd"))
        .map(|service| {
            let container_name = service.0;
            let lnd_name = container_name.split('-').last().unwrap();
            let mut found_ip: Option<_> = None;
            if let Networks::Advanced(AdvancedNetworks(networks)) = service.1.unwrap().networks {
                if let MapOrEmpty::Map(advance_setting) = networks.first().unwrap().1 {
                    found_ip = advance_setting.ipv4_address.clone();
                }
            }
            get_existing_lnd_config(lnd_name, container_name.clone(), found_ip.unwrap())
        })
        .filter_map(|res| res.ok())
        .collect();
    let logger = options.global_logger();
    let compose_path = options.compose_path.clone().unwrap();
    lnds.iter_mut().for_each(|node| {
        let compose_path_clone = compose_path.clone();
        let result = get_node_info(
            options.docker_command.clone(),
            &logger,
            node,
            compose_path_clone.clone(),
        );

        match result {
            Ok(_) => info!(logger, "container: {} found", node.container_name.clone()),
            Err(e) => error!(logger, "failed to find node: {}", e),
        }
    });

    options.lnds = lnds;

    Ok(())
}

fn get_existing_lnd_config(name: &str, container_name: String, ip: String) -> Result<Lnd, Error> {
    let full_path = &format!("data/{}/.lnd", name);
    Ok(Lnd {
        name: name.to_owned(),
        alias: name.to_owned(),
        container_name: container_name.to_owned(),
        pubkey: None,
        ip: ip.clone(),
        rpc_server: format!("{}:10000", ip),
        server_url: format!("https://{}:8080", ip),
        certificate_path: format!("{}/tls.crt", full_path),
        macaroon_path: format!(
            "{}/data/chain/bitcoin/{}/admin.macaroon",
            full_path, "regtest"
        ),
        path_vol: full_path.to_owned(),
        grpc_port: "10000".to_owned(),
        rest_port: "8080".to_owned(),
        p2p_port: "9735".to_owned(),
    })
}

fn set_bitcoind_values(conf: &mut FileConf, bitcoind_node: &Bitcoind) -> Result<(), Error> {
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
            bitcoind_node.ip, &bitcoind_node.zmqpubrawblock
        )
        .as_str(),
    );
    bitcoind.set_property(
        "bitcoind.zmqpubrawtx",
        format!("tcp://{}:{}", bitcoind_node.ip, &bitcoind_node.zmqpubrawtx).as_str(),
    );
    bitcoind.set_property("bitcoind.rpcpass", &bitcoind_node.password);
    bitcoind.set_property("bitcoind.rpcuser", &bitcoind_node.user);
    bitcoind.set_property(
        "bitcoind.rpchost",
        format!("{}:{}", bitcoind_node.ip, &bitcoind_node.rpcport).as_str(),
    );

    Ok(())
}

fn set_application_options_values(conf: &mut FileConf, name: &str, ip: &str) -> Result<(), Error> {
    if conf.sections.get("Application Options").is_none() {
        conf.sections
            .insert("Application Options".to_owned(), Section::new());
    }
    let application_options = conf.sections.get_mut("Application Options").unwrap();
    application_options.set_property("alias", name);
    application_options.set_property("tlsextradomain", name);
    application_options.set_property("tlsextraip", ip);
    application_options.set_property("restlisten", &format!("{}:8080", ip));
    application_options.set_property("rpclisten", &format!("{}:10000", ip));
    Ok(())
}

pub fn get_node_info(
    docker_command: String,
    logger: &Logger,
    lnd: &mut Lnd,
    compose_path: String,
) -> Result<(), Error> {
    let mut output_found = None;
    let mut retries = 3;
    let rpc_command = lnd.get_rpc_server_command();
    let macaroon_path = lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "getinfo",
    ];
    while retries > 0 {
        let output = run_command(
            logger,
            docker_command.clone(),
            "pubkey".to_owned(),
            commands.clone(),
        )?;
        if from_utf8(&output.stderr)?.contains("not running") {
            debug!(logger, "trying to get pubkey again");
            thread::sleep(Duration::from_secs(1));
            retries -= 1;
        } else {
            output_found = Some(output);
            break;
        }
    }
    let output = output_found.unwrap();
    if output.status.success() {
        if let Some(pubkey) = get_property("identity_pubkey", output.clone()) {
            lnd.pubkey = Some(pubkey);
        } else {
            error!(logger, "no pubkey found");
        }
    }
    Ok(())
}

pub fn fund_node(
    docker_command: String,
    logger: &Logger,
    lnd: &mut Lnd,
    miner: &Bitcoind,
    compose_path: String,
) -> Result<(), Error> {
    create_lnd_wallet(logger, docker_command.clone(), lnd, compose_path.clone())?;
    let address = create_lnd_address(logger, docker_command.clone(), lnd, compose_path.clone())?;
    mine_to_address(
        logger,
        docker_command,
        compose_path,
        miner.container_name.clone(),
        miner.data_dir.clone(),
        2,
        address,
    )?;
    Ok(())
}

pub fn create_lnd_wallet(
    logger: &Logger,
    docker_command: String,
    lnd: &mut Lnd,
    compose_path: String,
) -> Result<(), Error> {
    let rpc_command = lnd.get_rpc_server_command();
    let macaroon_path = lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "createwallet",
    ];
    let output = run_command(logger, docker_command, "createwallet".to_owned(), commands)?;
    if output.status.success() {
        let _response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
    }
    Ok(())
}

pub fn create_lnd_address(
    logger: &Logger,
    docker_command: String,
    lnd: &mut Lnd,
    compose_path: String,
) -> Result<String, Error> {
    let rpc_command = lnd.get_rpc_server_command();
    let macaroon_path = lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "newaddress",
        "p2tr", // TODO: set as a taproot address by default, make this configurable
    ];
    let output = run_command(logger, docker_command, "newaddress".to_owned(), commands)?;
    let found_address: Option<String> = get_property("address", output.clone());
    if found_address.is_none() {
        error!(logger, "no addess found");
    }
    Ok(found_address.unwrap())
}

#[derive(Default, Debug, Clone)]
pub struct NodeCommand {
    pub name: String,
    pub from: String,
    pub to: String,
    pub amt: Option<i64>,
    pub subcommand: Option<String>,
}

pub fn open_channel(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let _ = connect(options, node_command).map_err(|e| {
        debug!(options.global_logger(), "failed to connect: {}", e);
    });
    let to_lnd = get_lnd_by_name(options, node_command.to.as_str())?;
    let from_lnd = get_lnd_by_name(options, node_command.from.as_str())?;
    let amt = node_command.amt.unwrap_or(100000).to_string();
    let rpc_command = from_lnd.get_rpc_server_command();
    let macaroon_path = from_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        from_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "openchannel",
        to_lnd.pubkey.as_ref().unwrap().as_ref(),
        amt.as_ref(),
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "openchannel".to_owned(),
        commands,
    )?;
    if output.status.success() {
        info!(
            options.global_logger(),
            "successfully opened channel from {} to {}", from_lnd.name, to_lnd.name
        );
    } else {
        error!(
            options.global_logger(),
            "failed to open channel from {} to {}", from_lnd.name, to_lnd.name
        );
    }
    Ok(())
}

pub fn connect(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let to_lnd: &Lnd = get_lnd_by_name(options, node_command.to.as_str())?;
    let from_lnd = get_lnd_by_name(options, node_command.from.as_str())?;
    let connection_url = to_lnd.get_connection_url();
    let rpc_command = from_lnd.get_rpc_server_command();
    let macaroon_path = from_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        from_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "connect",
        connection_url.as_ref(),
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "connect".to_owned(),
        commands,
    )?;

    if output.status.success() || from_utf8(&output.stderr)?.contains("already connected to peer") {
        info!(
            options.global_logger(),
            "successfully connected from {} to {}", from_lnd.name, to_lnd.name
        );
    } else {
        error!(
            options.global_logger(),
            "failed to connect from {} to {}", from_lnd.name, to_lnd.name
        );
    }
    Ok(())
}

pub fn close_channel(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let peer_channel_point = get_peers_channels(options, node_command)?;
    let from_lnd = get_lnd_by_name(options, node_command.from.as_str())?;
    let to_lnd: &Lnd = get_lnd_by_name(options, node_command.to.as_str())?;
    let rpc_command = from_lnd.get_rpc_server_command();
    let macaroon_path = from_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        from_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "closechannel",
        "--chan_point",
        peer_channel_point.as_ref(),
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "closechannel".to_owned(),
        commands,
    )?;

    if output.status.success() {
        info!(
            options.global_logger(),
            "successfully closed channel from {} to {}", from_lnd.name, to_lnd.name
        );
    } else {
        error!(
            options.global_logger(),
            "failed to close channel from {} to {}", from_lnd.name, to_lnd.name
        );
    }
    Ok(())
}

pub fn get_peers_channels(options: &Options, node_command: &NodeCommand) -> Result<String, Error> {
    let to_lnd = get_lnd_by_name(options, node_command.to.as_str())?;
    let from_lnd = get_lnd_by_name(options, node_command.from.as_str())?;
    let rpc_command = from_lnd.get_rpc_server_command();
    let macaroon_path = from_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        from_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "listchannels",
        "--peer",
        to_lnd.pubkey.as_ref().unwrap().as_ref(),
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "listchannels".to_owned(),
        commands,
    )?;
    let channel_point = get_array_property("channels", "channel_point", output);
    if channel_point.is_none() {
        return Err(anyhow!("no channel point found!"));
    }
    Ok(channel_point.unwrap())
}

pub fn send_ln(options: &mut Options, node_command: &NodeCommand) -> Result<(), Error> {
    let invoice = create_invoice(options, node_command)?;
    pay_invoice(options, node_command, invoice)?;
    Ok(())
}

pub fn send_on_chain(options: &mut Options, node_command: &NodeCommand) -> Result<(), Error> {
    let on_chain_address_from = create_on_chain_address(options, node_command)?;
    let tx_id = pay_address(options, node_command, on_chain_address_from.as_str())?;
    info!(
        options.global_logger(),
        "on chain transaction created: {}", tx_id
    );
    Ok(())
}

fn generate_memo() -> String {
    let words = [
        "piano",
        "balance",
        "transaction",
        "exchange",
        "receipt",
        "wire",
        "deposit",
        "wallet",
        "sats",
        "profit",
        "transfer",
        "vendor",
        "investment",
        "payment",
        "debit",
        "card",
        "bank",
        "account",
        "money",
        "order",
        "gateway",
        "online",
        "confirmation",
        "interest",
        "fraud",
        "Olivia",
        "Elijah",
        "Ava",
        "Liam",
        "Isabella",
        "Mason",
        "Sophia",
        "William",
        "Emma",
        "James",
        "parrot",
        "dolphin",
        "breeze",
        "moonlight",
        "whisper",
        "velvet",
        "marble",
        "sunset",
        "seashell",
        "peacock",
        "rainbow",
        "guitar",
        "harmony",
        "lulla",
        "crystal",
        "butterfly",
        "stardust",
        "cascade",
        "serenade",
        "lighthouse",
        "orchid",
        "sapphire",
        "silhouette",
        "tulip",
        "firefly",
        "brook",
        "feather",
        "mermaid",
        "twilight",
        "dandelion",
        "morning",
        "serenity",
        "emerald",
        "flamingo",
        "gazelle",
        "ocean",
        "carousel",
        "sparkle",
        "dewdrop",
        "paradise",
        "polaris",
        "meadow",
        "quartz",
        "zenith",
        "horizon",
        "sunflower",
        "melody",
        "trinket",
        "whisker",
        "cabana",
        "harp",
        "blossom",
        "jubilee",
        "raindrop",
        "sunrise",
        "zeppelin",
        "whistle",
        "ebony",
        "gardenia",
        "lily",
        "marigold",
        "panther",
        "starlight",
        "harmonica",
        "shimmer",
        "canary",
        "comet",
        "moonstone",
        "rainforest",
        "buttercup",
        "zephyr",
        "violet",
        "serenade",
        "swan",
        "pebble",
        "coral",
        "radiance",
        "violin",
        "zodiac",
        "serenade",
    ];

    let mut rng = rand::thread_rng();
    let random_index = rng.gen_range(0..words.len());
    let mut memo = String::new();
    let limit = rng.gen_range(1..=15);
    for (index, word) in words.iter().enumerate() {
        if index >= limit {
            break;
        }
        if !memo.is_empty() {
            memo.push(' ');
        }
        memo.push_str(word);
    }
    let random_word = words[random_index];
    random_word.to_owned()
}

pub fn create_invoice(options: &mut Options, node_command: &NodeCommand) -> Result<String, Error> {
    let to_lnd = get_lnd_by_name(options, node_command.to.as_str())?;
    let amt = node_command.amt.unwrap_or(1000).to_string();
    let memo = generate_memo();
    let rpc_command = to_lnd.get_rpc_server_command();
    let macaroon_path = to_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        to_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "addinvoice",
        "--memo",
        memo.as_ref(),
        "--amt",
        amt.as_ref(),
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "addinvoice".to_owned(),
        commands,
    )?;
    let found_payment_request: Option<String> = get_property("payment_request", output.clone());
    if found_payment_request.is_none() {
        error!(options.global_logger(), "no payment hash found");
    }
    Ok(found_payment_request.unwrap())
}

pub fn pay_invoice(
    options: &mut Options,
    node_command: &NodeCommand,
    payment_request: String,
) -> Result<(), Error> {
    let from_lnd = get_lnd_by_name(options, node_command.from.as_str())?;
    let rpc_command = from_lnd.get_rpc_server_command();
    let macaroon_path = from_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        from_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "payinvoice",
        "-f",
        payment_request.as_ref(),
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "payinvoice".to_owned(),
        commands,
    )?;
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

pub fn get_property(name: &str, output: Output) -> Option<String> {
    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        if let Some(value) = response
            .as_mapping()
            .and_then(|obj| obj.get(name))
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            return Some(value);
        } else {
            return None;
        }
    }
    None
}

pub fn get_array_property(
    array_name: &str,
    inner_property: &str,
    output: Output,
) -> Option<String> {
    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        if let Some(value) = response
            .as_mapping()
            .and_then(|obj| obj.get(array_name))
            .and_then(|array| array.as_sequence())
            .and_then(|array| array.first())
            .and_then(|obj| obj.get(inner_property))
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            return Some(value);
        } else {
            return None;
        }
    }
    None
}

pub fn get_lnd_by_name<'a>(options: &'a Options, name: &str) -> Result<&'a Lnd, Error> {
    let lnd = options
        .lnds
        .iter()
        .find(|lnd| lnd.name == *name)
        .unwrap_or_else(|| panic!("invalid lnd node name to: {:?}", name));
    Ok(lnd)
}

pub fn create_on_chain_address(
    options: &mut Options,
    node_command: &NodeCommand,
) -> Result<String, Error> {
    let to_lnd: &Lnd = get_lnd_by_name(options, node_command.to.as_str())?;
    let rpc_command = to_lnd.get_rpc_server_command();
    let macaroon_path = to_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        to_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "newaddress",
        "p2tr", //TODO: allow for other types beside taproot addresses
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "newaddress".to_owned(),
        commands,
    )?;
    let found_address: Option<String> = get_property("address", output.clone());
    if found_address.is_none() {
        error!(options.global_logger(), "no on chain address found");
    }
    debug!(
        options.global_logger(),
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(found_address.unwrap())
}

pub fn pay_address(
    options: &mut Options,
    node_command: &NodeCommand,
    address: &str,
) -> Result<String, Error> {
    let to_lnd = get_lnd_by_name(options, node_command.to.as_str())?;
    let amt = node_command.amt.unwrap_or(1000).to_string();
    let subcommand = node_command.subcommand.to_owned().unwrap_or("".to_owned());
    let rpc_command = to_lnd.get_rpc_server_command();
    let macaroon_path = to_lnd.get_macaroon_command();
    let commands = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "exec",
        "--user",
        "1000:1000",
        to_lnd.container_name.as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        &macaroon_path,
        &rpc_command,
        "sendcoins",
        subcommand.as_ref(),
        "--addr",
        address,
        "--amt",
        amt.as_ref(),
    ];
    let output = run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "sendcoins".to_owned(),
        commands,
    )?;
    let found_tx_id: Option<String> = get_property("txid", output.clone());
    if found_tx_id.is_none() {
        error!(options.global_logger(), "no tx id found");
        return Ok("".to_owned());
    }

    Ok(found_tx_id.unwrap())
}
