use crate::{
    copy_file, get_absolute_path, run_command, ImageInfo, L1Node, NodeCommand,
    Options, NETWORK,
};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{FileConf, Section};
use docker_compose_types::{EnvFile, Networks, Ports, Service, Volumes};
use slog::{error, info, Logger};
use std::{
    fs::{File, OpenOptions},
    str::from_utf8,
};

#[derive(Default, Debug, Clone)]
pub struct Bitcoind {
    pub conf: FileConf,
    pub data_dir: String,
    pub container_name: String,
    pub name: String,
    pub p2pport: String,
    pub rpcport: String,
    pub user: String,
    pub password: String,
    pub zmqpubrawblock: String,
    pub zmqpubhashblock: String,
    pub zmqpubrawtx: String,
    pub path_vol: String,
}

pub enum L1Enum {
    L1Node(Bitcoind),
}

impl L1Node for Bitcoind {
    fn get_name(&self) -> String {
        self.name.clone()
    }
    fn get_container_name(&self) -> String {
        self.container_name.clone()
    }
    fn get_data_dir(&self) -> String {
        self.data_dir.clone()
    }
    fn get_p2p_port(&self) -> String {
        self.p2pport.clone()
    }
    fn get_zmqpubrawblock(&self) -> String {
        self.zmqpubrawblock.clone()
    }
    fn get_zmqpubhashblock(&self) -> String {
        self.zmqpubhashblock.clone()
    }
    fn get_zmqpubrawtx(&self) -> String {
        self.zmqpubrawtx.clone()
    }
    fn get_rpc_username(&self) -> String {
        self.user.clone()
    }
    fn get_rpc_password(&self) -> String {
        self.password.clone()
    }
    fn get_rpc_port(&self) -> String {
        self.rpcport.clone()
    }
    fn mine_bitcoin(&self, options: &Options, num_blocks: i64) -> Result<String, Error> {
        mine_bitcoin(self.clone(), options, num_blocks)
    }
    fn create_wallet(&self, options: &Options) -> Result<(), Error> {
        create_wallet(self, options)
    }
    fn load_wallet(&self, options: &Options) -> Result<(), Error> {
        load_wallet(self, options)
    }
    fn create_address(&self, options: &Options) -> Result<String, Error> {
        create_address(self, options)
    }
    fn mine_to_address(
        self,
        options: &Options,
        num_blocks: i64,
        address: String,
    ) -> Result<(), Error> {
        mine_to_address(self, options, num_blocks, address)
    }
    fn send_to_l2(self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        let to = options.get_l2_by_name(&node_command.to)?;
        let address = to.create_on_chain_address(options)?;

        //default to sending 100,000 sats
        self.send_to_address(options, 1, node_command.amt.unwrap_or(100000), address)?;
        Ok(())
    }
    fn send_to_address(
        self,
        options: &Options,
        num_blocks: i64,
        amt: i64,
        address: String,
    ) -> Result<(), Error> {
        send_to_address(self, options, num_blocks, amt, address)
    }
}

pub fn get_config(options: &mut Options, name: &str, is_miner: bool) -> Result<Bitcoind, Error> {
    get_bitcoind_config(options, name, is_miner)
}
pub fn add_config(
    options: &Options,
    name: &str,
    network: &str,
    container_name: &str,
) -> Result<Bitcoind, Error> {
    load_config(name, container_name, network, options.global_logger())
}

pub fn build_bitcoind(
    options: &mut Options,
    name: &str,
    image: &ImageInfo,
    is_miner: bool,
) -> Result<()> {
    let bitcoind_conf = get_config(options, name, is_miner).unwrap();
    let bitcoind = Service {
        image: Some(image.get_image()),
        container_name: Some(bitcoind_conf.container_name.clone()),
        ports: Ports::Short(vec![
            format!("{}:{}", options.new_port(), bitcoind_conf.p2pport),
            format!("{}:{}", options.new_port(), bitcoind_conf.rpcport),
        ]),
        volumes: Volumes::Simple(vec![format!(
            "{}:/home/bitcoin/.bitcoin:rw",
            bitcoind_conf.path_vol
        )]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        networks: Networks::Simple(vec![NETWORK.to_owned()]),
        ..Default::default()
    };
    options
        .services
        .insert(bitcoind_conf.container_name.clone(), Some(bitcoind));
    options.bitcoinds.push(bitcoind_conf);
    Ok(())
}

pub fn add_bitcoinds(options: &mut Options) -> Result<()> {
    let logger: Logger = options.global_logger();
    let bitcoinds: Vec<_> = options
        .services
        .iter_mut()
        .filter(|service| service.0.contains("bitcoind"))
        .map(|service| {
            let container_name = service.0;
            let bitcoind_name = container_name.split('-').last().unwrap();
            load_config(
                bitcoind_name,
                container_name.as_str(),
                &options.network,
                logger.clone(),
            )
        })
        .filter_map(|res| res.ok())
        .collect();
    options.bitcoinds = bitcoinds;
    Ok(())
}

fn load_config(
    name: &str,
    container_name: &str,
    network: &str,
    logger: Logger,
) -> Result<Bitcoind, Error> {
    let bitcoind_config: &String = &format!("data/{}/.bitcoin/bitcoin.conf", name);
    let full_path = get_absolute_path(bitcoind_config)?
        .to_str()
        .unwrap()
        .to_string();
    let source: File = OpenOptions::new()
        .read(true)
        .write(true)
        .open(full_path.clone())?;
    let conf = conf_parser::processer::read_to_file_conf_mut(&source).map_err(|e| {
        error!(logger, "failed to read bitcoind conf file: {}", e);
        e
    })?;
    let network_section = get_network_section(conf, network).map_err(|e| {
        error!(
            logger,
            "failed to get network section from bitcoind conf file: {}", e
        );
        e
    })?;

    Ok(Bitcoind {
        conf: conf.to_owned(),
        name: name.to_owned(),
        data_dir: "/home/bitcoin/.bitcoin".to_owned(),
        container_name: container_name.to_owned(),
        path_vol: full_path,
        user: network_section.get_property("rpcuser"),
        password: network_section.get_property("rpcpassword"),
        p2pport: network_section.get_property("port"),
        rpcport: network_section.get_property("rpcport"),
        zmqpubrawblock: network_section
            .get_property("zmqpubrawblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubhashblock: network_section
            .get_property("zmqpubhashblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubrawtx: network_section
            .get_property("zmqpubrawtx")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
    })
}

fn get_bitcoind_config(
    options: &mut Options,
    name: &str,
    is_miner: bool,
) -> Result<Bitcoind, Error> {
    let original = get_absolute_path(&format!("config/{}/bitcoin.conf", options.network))?;
    let source: File = File::open(original)?;

    let destination_dir: &String = &format!("data/{}/.bitcoin", name);
    let conf = conf_parser::processer::read_to_file_conf_mut(&source)?;
    let network_section = set_network_section(conf, options)?;
    let _ = copy_file(conf, destination_dir, "bitcoin.conf")?;

    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();
    let container_name = match is_miner {
        true => format!("doppler-bitcoind-miner-{}", name),
        false => format!("doppler-bitcoind-{}", name),
    };
    Ok(Bitcoind {
        conf: conf.to_owned(),
        name: name.to_owned(),
        data_dir: "/home/bitcoin/.bitcoin".to_owned(),
        container_name,
        path_vol: full_path,
        user: network_section.get_property("rpcuser"),
        password: network_section.get_property("rpcpassword"),
        p2pport: network_section.get_property("port"),
        rpcport: network_section.get_property("rpcport"),
        zmqpubrawblock: network_section
            .get_property("zmqpubrawblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubhashblock: network_section
            .get_property("zmqpubhashblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubrawtx: network_section
            .get_property("zmqpubrawtx")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
    })
}

fn set_network_section(conf: &mut FileConf, options: &mut Options) -> Result<Section, Error> {
    if conf.sections.get(&options.network).is_none() {
        conf.sections
            .insert(options.network.clone(), Section::new());
    }
    let bitcoin = conf.sections.get_mut(&options.network).unwrap();
    let port = options.new_port();
    let rpc_port = options.new_port();
    bitcoin.set_property("bind", "0.0.0.0");
    bitcoin.set_property("port", &port.to_string());
    bitcoin.set_property("rpcport", &rpc_port.to_string());
    bitcoin.set_property("rpcuser", "admin");
    bitcoin.set_property("rpcpassword", "1234");
    bitcoin.set_property(
        "zmqpubrawblock",
        &format!("tcp://0.0.0.0:{}", options.new_port()),
    );
    bitcoin.set_property(
        "zmqpubrawtx",
        &format!("tcp://0.0.0.0:{}", options.new_port()),
    );
    bitcoin.set_property(
        "zmqpubhashblock",
        &format!("tcp://0.0.0.0:{}", options.new_port()),
    );
    let network_section = get_network_section(conf, &options.network)?;
    Ok(network_section)
}

fn get_network_section(conf: &mut FileConf, network: &str) -> Result<Section, Error> {
    let network_section = conf.sections.get(network).expect("network section missing");
    Ok(network_section.to_owned())
}

pub fn pair_bitcoinds(options: &Options) -> Result<(), Error> {
    let options_clone = options;
    options
        .services
        .iter()
        .filter(|combo| combo.0.contains("bitcoind"))
        .for_each(|(name, _bitcoind_service)| {
            let mut listen_to = vec![];
            let current_bitcoind = options_clone
                .get_bitcoind_by_name(name.split('-').last().unwrap())
                .expect("unable to find L1 node by name");
            match current_bitcoind.create_wallet(&options.clone()) {
                Ok(_) => (),
                Err(e) => error!(
                    options.global_logger(),
                    "container {} failed to create wallet: {}",
                    current_bitcoind.get_container_name(),
                    e
                ),
            }
            options
                .bitcoinds
                .iter()
                .filter(|bitcoind| !bitcoind.get_container_name().eq_ignore_ascii_case(name))
                .for_each(|announce| {
                    let add_node = format!(
                        r#"{}:{}"#,
                        announce.get_container_name(),
                        announce.get_p2p_port()
                    );
                    listen_to.push(add_node)
                });
            pair_node(options_clone, current_bitcoind, listen_to).expect("failed to add nodes");
        });

    Ok(())
}

fn mine_bitcoin(node: impl L1Node, options: &Options, num_blocks: i64) -> Result<String, Error> {
    let address = node.create_address(options)?;

    node.mine_to_address(options, num_blocks, address.to_owned())?;

    Ok(address)
}

fn create_wallet(node: &dyn L1Node, options: &Options) -> Result<(), Error> {
    let datadir_flag = &format!("--datadir={}", node.get_data_dir());
    let container_name = node.get_container_name();
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &container_name,
        "bitcoin-cli",
        datadir_flag,
        "createwallet",
        &container_name,
    ];

    let output = run_command(options, "create wallet".to_owned(), commands)?;
    if !output.status.success()
        && !from_utf8(&output.stderr)
            .unwrap()
            .contains("Database already exists")
    {
        node.load_wallet(options)?;
    }
    Ok(())
}

fn load_wallet(node: &Bitcoind, options: &Options) -> Result<(), Error> {
    let datadir_flag = &format!("--datadir={}", node.get_data_dir());
    let container_name = node.get_container_name();
    let compose_path = options.compose_path.as_ref().unwrap();
    let commands = vec![
        "-f",
        compose_path,
        "exec",
        "--user",
        "1000:1000",
        &container_name,
        "bitcoin-cli",
        datadir_flag,
        "loadwallet",
        &container_name,
    ];

    let output = run_command(options, "load wallet".to_owned(), commands)?;
    if !output.status.success() {
        error!(
            options.global_logger(),
            "failed to load wallet for {}: {} {}",
            node.get_name(),
            from_utf8(&output.stdout).unwrap().to_owned(),
            from_utf8(&output.stderr).unwrap().to_owned()
        )
    }
    Ok(())
}

fn create_address(node: &Bitcoind, options: &Options) -> Result<String, Error> {
    let rpcwallet_flag = &format!("-rpcwallet={}", node.container_name);
    let datadir_flag = &format!("--datadir={}", node.data_dir);
    let compose_path = options.compose_path.clone().unwrap();

    let commands = vec![
        "-f",
        &compose_path,
        "exec",
        "--user",
        "1000:1000",
        &node.container_name,
        "bitcoin-cli",
        rpcwallet_flag,
        datadir_flag,
        "getnewaddress",
    ];
    let mut output = run_command(options, "getnewaddress".to_owned(), commands)?;
    if !output.status.success() {
        return Err(anyhow!("failed to create new address"));
    }
    // drop the newline character
    output.stdout.pop();
    let address = from_utf8(&output.stdout)?.to_owned();
    Ok(address)
}

fn send_to_address(
    node: impl L1Node,
    options: &Options,
    num_blocks: i64,
    amt: i64,
    address: String,
) -> Result<(), Error> {
    if amt == 0 {
        return Ok(());
    }
    let datadir_flag = &format!("--datadir={}", node.get_data_dir());
    let container_name = node.get_container_name();
    let compose_path = options.compose_path.clone().unwrap();
    let amt_btc = ((amt as f64) / 100_000_000_f64).to_string();
    let commands = vec![
        "-f",
        &compose_path,
        "exec",
        "--user",
        "1000:1000",
        &container_name,
        "bitcoin-cli",
        datadir_flag,
        "sendtoaddress",
        &address,
        &amt_btc,
    ];
    let output = run_command(options, "getnewaddress".to_owned(), commands)?;
    if !output.status.success() {
        error!(
            options.global_logger(),
            "failed to mine to address: {} {}",
            from_utf8(&output.stdout).unwrap().to_owned(),
            from_utf8(&output.stderr).unwrap().to_owned()
        );
    }
    mine_bitcoin(node, options, num_blocks)?;
    Ok(())
}

fn mine_to_address(
    node: impl L1Node,
    options: &Options,
    num_blocks: i64,
    address: String,
) -> Result<(), Error> {
    let datadir_flag = &format!("--datadir={}", node.get_data_dir());
    let block_arg = &num_blocks.to_string();
    let container_name = node.get_container_name();
    let compose_path = options.compose_path.clone().unwrap();

    let commands = vec![
        "-f",
        &compose_path,
        "exec",
        "--user",
        "1000:1000",
        &container_name,
        "bitcoin-cli",
        datadir_flag,
        "generatetoaddress",
        block_arg,
        &address,
    ];
    let output = run_command(options, "getnewaddress".to_owned(), commands)?;
    if !output.status.success() {
        error!(
            options.global_logger(),
            "failed to mine to address: {} {}",
            from_utf8(&output.stdout).unwrap().to_owned(),
            from_utf8(&output.stderr).unwrap().to_owned()
        );
    }
    Ok(())
}

fn pair_node(
    options: &Options,
    current_node: &dyn L1Node,
    nodes: Vec<String>,
) -> Result<(), Error> {
    let compose_path = options.compose_path.clone().unwrap();
    let datadir_flag = &format!("--datadir={}", current_node.get_data_dir());

    for node in nodes.iter() {
        let container_name = current_node.get_container_name();
        // -rpcport="3133" -rpcuser="alice" -rpcpassword="hello"
        let rpc_port = format!("-rpcport={}", current_node.get_rpc_port());
        let rpc_user = format!("-rpcuser={}", current_node.get_rpc_username());
        let rpc_password = format!("-rpcpassword={}", current_node.get_rpc_password());

        let commands = vec![
            "-f",
            compose_path.as_ref(),
            "exec",
            "--user",
            "1000:1000",
            &container_name,
            "bitcoin-cli",
            datadir_flag,
            &rpc_port,
            &rpc_user,
            &rpc_password,
            "addnode",
            node,
            r#"add"#,
        ];
        run_command(options, "addnode".to_owned(), commands)?;
    }

    Ok(())
}
