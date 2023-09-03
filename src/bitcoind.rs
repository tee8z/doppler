use crate::{copy_file, get_absolute_path, run_command, L1Node, MinerTime, Options, NETWORK};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{FileConf, Section};
use docker_compose_types::{
    AdvancedNetworkSettings, AdvancedNetworks, EnvFile, MapOrEmpty, Networks, Ports, Service,
    Volumes,
};
use indexmap::IndexMap;
use slog::{error, Logger};
use std::{
    fs::{File, OpenOptions},
    str::from_utf8,
    thread,
    thread::spawn,
    time::Duration,
};

const BITCOIND_IMAGE: &str = "polarlightning/bitcoind:25.0";

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
    pub ip: String,
    pub miner_time: Option<MinerTime>,
}

pub enum L1Enum {
    L1Node(Bitcoind),
}

impl L1Node for Bitcoind {
    fn start_mining(&self, options: &Options) -> Result<()> {
        start_mining(self, options)
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
    fn get_container_name(&self) -> String {
        self.container_name.clone()
    }
    fn get_data_dir(&self) -> String {
        self.data_dir.clone()
    }
    fn get_miner_time(&self) -> &Option<MinerTime> {
        &self.miner_time
    }
    fn get_ip(&self) -> String {
        self.ip.clone()
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
    fn mine_bitcoin_continously(&self, options: &Options) {
        mine_bitcoin_continously(self, options)
    }
    fn mine_bitcoin(&self, options: &Options, num_blocks: i64) -> Result<String, Error> {
        mine_bitcoin(self, options, num_blocks)
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
}

pub fn get_config(
    options: &mut Options,
    name: &str,
    miner_time: &Option<MinerTime>,
    ip: &str,
) -> Result<Bitcoind, Error> {
    get_bitcoind_config(options, name, miner_time, ip)
}
pub fn add_config(
    options: &Options,
    name: &str,
    container_name: &str,
    ip: &str,
) -> Result<Bitcoind, Error> {
    load_config(name, container_name, options.global_logger(), ip)
}

pub fn build_bitcoind(
    options: &mut Options,
    name: &str,
    miner_time: &Option<MinerTime>,
) -> Result<()> {
    let ip = options.new_ipv4().to_string();
    let bitcoind_conf = get_config(options, name, miner_time, ip.as_str()).unwrap();
    let mut cur_network = IndexMap::new();
    cur_network.insert(
        NETWORK.to_string(),
        MapOrEmpty::Map(AdvancedNetworkSettings {
            ipv4_address: Some(ip),
            ..Default::default()
        }),
    );

    let bitcoind = Service {
        image: Some(BITCOIND_IMAGE.to_string()),
        container_name: Some(bitcoind_conf.container_name.clone()),
        ports: Ports::Short(vec![
            format!("{}", bitcoind_conf.p2pport),
            format!("{}", bitcoind_conf.rpcport),
        ]),
        volumes: Volumes::Simple(vec![format!(
            "{}:/home/bitcoin/.bitcoin:rw",
            bitcoind_conf.path_vol
        )]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        networks: Networks::Advanced(AdvancedNetworks(cur_network)),
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
            let mut found_ip: Option<_> = None;
            if let Networks::Advanced(AdvancedNetworks(networks)) =
                service.1.as_ref().unwrap().networks.clone()
            {
                if let MapOrEmpty::Map(advance_setting) = networks.first().unwrap().1 {
                    found_ip = advance_setting.ipv4_address.clone();
                }
            }
            load_config(
                bitcoind_name,
                container_name.as_str(),
                logger.clone(),
                found_ip.unwrap().as_str(),
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
    logger: Logger,
    ip: &str,
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
    let regtest_section = get_regtest_section(conf).map_err(|e| {
        error!(
            logger,
            "failed to get regtest section from bitcoind conf file: {}", e
        );
        e
    })?;

    Ok(Bitcoind {
        conf: conf.to_owned(),
        ip: ip.to_owned(),
        name: name.to_owned(),
        data_dir: "/home/bitcoin/.bitcoin".to_owned(),
        miner_time: None,
        container_name: container_name.to_owned(),
        path_vol: full_path,
        user: regtest_section.get_property("rpcuser"),
        password: regtest_section.get_property("rpcpassword"),
        p2pport: regtest_section.get_property("port"),
        rpcport: regtest_section.get_property("rpcport"),
        zmqpubrawblock: regtest_section
            .get_property("zmqpubrawblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubhashblock: regtest_section
            .get_property("zmqpubhashblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubrawtx: regtest_section
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
    miner_time: &Option<MinerTime>,
    ip: &str,
) -> Result<Bitcoind, Error> {
    let original = get_absolute_path("config/bitcoin.conf")?;
    let source: File = File::open(original)?;

    let destination_dir: &String = &format!("data/{}/.bitcoin", name);
    let conf = conf_parser::processer::read_to_file_conf_mut(&source)?;
    let regtest_section = set_regtest_section(conf, options, ip)?;
    let _ = copy_file(conf, destination_dir, "bitcoin.conf")?;

    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();
    let container_name = match miner_time.is_some() {
        true => format!("doppler-bitcoind-miner-{}", name),
        false => format!("doppler-bitcoind-{}", name),
    };
    Ok(Bitcoind {
        conf: conf.to_owned(),
        ip: ip.to_owned(),
        name: name.to_owned(),
        data_dir: "/home/bitcoin/.bitcoin".to_owned(),
        miner_time: miner_time.to_owned(),
        container_name,
        path_vol: full_path,
        user: regtest_section.get_property("rpcuser"),
        password: regtest_section.get_property("rpcpassword"),
        p2pport: regtest_section.get_property("port"),
        rpcport: regtest_section.get_property("rpcport"),
        zmqpubrawblock: regtest_section
            .get_property("zmqpubrawblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubhashblock: regtest_section
            .get_property("zmqpubhashblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubrawtx: regtest_section
            .get_property("zmqpubrawtx")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
    })
}

fn set_regtest_section(
    conf: &mut FileConf,
    options: &mut Options,
    ip: &str,
) -> Result<Section, Error> {
    if conf.sections.get("regtest").is_none() {
        conf.sections.insert("regtest".to_owned(), Section::new());
    }
    let bitcoin = conf.sections.get_mut("regtest").unwrap();
    let port = options.new_port();
    let rpc_port = options.new_port();
    bitcoin.set_property("bind", ip);
    bitcoin.set_property("port", &port.to_string());
    bitcoin.set_property("rpcport", &rpc_port.to_string());
    bitcoin.set_property("rpcuser", "admin");
    bitcoin.set_property("rpcpassword", "1234");
    bitcoin.set_property(
        "zmqpubrawblock",
        &format!("tcp://{}:{}", ip, options.new_port()),
    );
    bitcoin.set_property(
        "zmqpubrawtx",
        &format!("tcp://{}:{}", ip, options.new_port()),
    );
    bitcoin.set_property(
        "zmqpubhashblock",
        &format!("tcp://{}:{}", ip, options.new_port()),
    );
    let regtest_section = get_regtest_section(conf)?;
    Ok(regtest_section)
}

fn get_regtest_section(conf: &mut FileConf) -> Result<Section, Error> {
    let regtest_section = conf
        .sections
        .get("regtest")
        .expect("regtest section missing");
    Ok(regtest_section.to_owned())
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

            options
                .bitcoinds
                .iter()
                .filter(|bitcoind| !bitcoind.get_container_name().eq_ignore_ascii_case(name))
                .for_each(|announce| {
                    let add_node = format!(r#"{}:{}"#, announce.get_ip(), announce.get_p2p_port());
                    listen_to.push(add_node)
                });
            pair_node(options_clone, current_bitcoind, listen_to).expect("failed to add nodes");
        });

    Ok(())
}

fn start_mining(node: &Bitcoind, options: &Options) -> Result<()> {
    let logger = options.clone().global_logger();
    let cloned_options = options.clone();
    let cloned_node = node.clone();
    match node.create_wallet(&options.clone()) {
        Ok(_) => (),
        Err(e) => error!(
            logger,
            "container {} failed to create wallet: {}",
            node.get_container_name(),
            e
        ),
    }

    spawn(move || {
        cloned_node.mine_bitcoin_continously(&cloned_options);
        let thread_handle = thread::current();
        cloned_options.add_thread(thread_handle);
    });
    Ok(())
}

fn mine_bitcoin_continously(node: &Bitcoind, option: &Options) {
    let miner_time = node.get_miner_time().as_ref().unwrap();
    let sleep_time = match miner_time.miner_interval_type {
        's' => Duration::from_secs(miner_time.miner_interval_amt),
        'm' => Duration::from_secs(miner_time.miner_interval_amt * 60),
        'h' => Duration::from_secs(miner_time.miner_interval_amt * 60 * 60),
        _ => unimplemented!(),
    };
    while option.main_thread_active.val() {
        thread::sleep(sleep_time);
        let thread_logger = option.global_logger();
        if !option.main_thread_paused.val() {
            match mine_bitcoin(node, option, 1) {
                Ok(_) => (),
                Err(e) => error!(
                    thread_logger,
                    "container {} failed to mine blocks: {}",
                    node.get_container_name().clone(),
                    e
                ),
            }
        }
    }
}
fn mine_bitcoin(node: &Bitcoind, options: &Options, num_blocks: i64) -> Result<String, Error> {
    let address = node.create_address(options)?;

    node.clone()
        .mine_to_address(options, num_blocks, address.to_owned())?;

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
        let rpc_password = format!("-rpcuser={}", current_node.get_rpc_username());

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
