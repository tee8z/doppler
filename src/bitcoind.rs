use crate::{copy_file, get_absolute_path, Bitcoind, Options, NETWORK, ThreadController};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{FileConf, Section};
use docker_compose_types::{Command, Networks, Ports, Service, Volumes, EnvFile};
use log::{debug, error, info};
use std::{fs::File, process, str::from_utf8, thread, time::Duration};

const BITCOIND_IMAGE: &str = "polarlightning/bitcoind:25.0";

#[derive(Debug, Default, Clone)]
pub struct MinerTime {
    pub miner_interval_amt: u64,
    pub miner_interval_type: char,
}

impl MinerTime {
    pub fn new(amt: u64, time_type: char) -> MinerTime {
        MinerTime {
            miner_interval_amt: amt,
            miner_interval_type: time_type,
        }
    }
}

pub fn build_bitcoind(
    options: &mut Options,
    name: &str,
    miner_time: Option<MinerTime>,
) -> Result<()> {
    let mut bitcoind_conf = get_bitcoind_config(options, name).unwrap();
    bitcoind_conf.miner_time = miner_time.clone();
    debug!("{} bitcoind vol: {}", name, bitcoind_conf.path_vol);

    let rpc_port = options.new_port();
    let container_name = match miner_time.is_some() {
        true => format!("doppler-bitcoind-miner-{}", name),
        false => format!("doppler-bitcoind-{}", name),
    };

    let bitcoind = Service {
        image: Some(BITCOIND_IMAGE.to_string()),
        container_name: Some(container_name.clone()),
        ports: Ports::Short(vec![format!("{}:{}", rpc_port, bitcoind_conf.rpcport)]),
        volumes: Volumes::Simple(vec![format!(
            "{}:/home/bitcoin/.bitcoin:rw",
            bitcoind_conf.path_vol
        )]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        networks: Networks::Simple(vec![NETWORK.to_string()]),
        ..Default::default()
    };
    options
        .services
        .insert(container_name.clone(), Some(bitcoind));
    bitcoind_conf.container_name = Some(container_name);
    options.bitcoinds.push(bitcoind_conf);

    Ok(())
}

pub fn get_bitcoind_config(options: &mut Options, name: &str) -> Result<Bitcoind, Error> {
    let original = get_absolute_path("config/bitcoin.conf")?;
    let source: File = File::open(original)?;

    let destination_dir: &String = &format!("data/{}/.bitcoin", name);
    let conf = conf_parser::processer::read_to_file_conf_mut(&source)?;
    let regtest_section = set_regtest_section(conf, options)?;
    let _ = copy_file(conf, &destination_dir.clone(), "bitcoin.conf")?;

    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();

    Ok(Bitcoind {
        conf: conf.to_owned(),
        name: Some(name.to_owned()),
        data_dir: "/home/bitcoin/.bitcoin".to_owned(),
        miner_time: None,
        container_name: None,
        path_vol: full_path,
        user: regtest_section.get_property("rpcuser"),
        password: regtest_section.get_property("rpcpassword"),
        rpchost: regtest_section.get_property("rpcport"),
        rpcport: regtest_section.get_property("rpcport"),
        zmqpubrawblock: regtest_section
            .get_property("zmqpubrawblock")
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

fn set_regtest_section(conf: &mut FileConf, options: &mut Options) -> Result<Section, Error> {
    if conf.sections.get("regtest").is_none() {
        conf.sections.insert("regtest".to_owned(), Section::new());
    }
    let bitcoin = conf.sections.get_mut("regtest").unwrap();
    let port = options.new_port();
    let rpc_port = options.new_port();

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
    let regtest_section = conf
        .sections
        .get("regtest")
        .expect("regtest section missing");
    Ok(regtest_section.to_owned())
}

pub fn pair_bitcoinds(options: &mut Options) -> Result<(), Error> {
    options
        .services
        .iter_mut()
        .filter(|combo| combo.0.contains("bitcoind"))
        .for_each(|(name, bitcoind_service)| {
            let mut listen_to = vec![];

            options
                .bitcoinds
                .iter()
                .filter(|bitcoind| {
                    !bitcoind
                        .container_name
                        .as_ref()
                        .unwrap()
                        .eq_ignore_ascii_case(name)
                })
                .for_each(|announce| {
                    let add_node = format!(
                        "-addnode={}:{}",
                        announce.container_name.as_ref().unwrap(),
                        announce.rpcport
                    );
                    listen_to.push(add_node)
                });

            bitcoind_service.as_mut().unwrap().command = Some(Command::Args(listen_to));
        });

    Ok(())
}

pub fn start_mining(main_thread: ThreadController, bitcoind: &Bitcoind, compose_path: &str) -> Result<()> {
    let datadir: String = bitcoind.data_dir.clone();
    let container = bitcoind.container_name.clone().unwrap();
    let miner_time = bitcoind.miner_time.clone().unwrap().to_owned();
    let compose_path = compose_path.to_string();
    match create_wallet(
        compose_path.clone(),
        container.clone(),
        datadir.clone(),
    ) {
        Ok(_) => (),
        Err(e) => error!(
            "container {} failed to create wallet: {}",
            container, e
        ),
    }
    let _thread_handle = thread::spawn(move || {
        mine_bitcoin_continously(main_thread, container, datadir, compose_path, miner_time);
    });
    Ok(())
}

fn mine_bitcoin_continously(
    main_thread: ThreadController,
    container_name: String,
    datadir: String,
    compose_path: String,
    miner_time: MinerTime,
) {
    let sleep_time = match miner_time.miner_interval_type {
        's' => Duration::from_secs(miner_time.miner_interval_amt),
        'm' => Duration::from_secs(miner_time.miner_interval_amt * 60),
        'h' => Duration::from_secs(miner_time.miner_interval_amt * 60 * 60),
        _ => unimplemented!(),
    };
    while !main_thread.is_terminated() {
        thread::sleep(sleep_time);
        match mine_bitcoin(
            compose_path.clone(),
            container_name.clone(),
            datadir.clone(),
            1,
        ) {
            Ok(_) => (),
            Err(e) => error!(
                "container {} failed to mine blocks: {}",
                container_name.clone(),
                e
            ),
        }
    }
}
pub fn mine_bitcoin(
    compose_path: String,
    container_name: String,
    datadir: String,
    num_blocks: i64,
) -> Result<String, Error> {
    let address = create_address(
        compose_path.clone(),
        container_name.clone(),
        datadir.clone(),
    )?;

    mine_to_address(
        compose_path,
        container_name,
        datadir,
        num_blocks,
        address.to_owned(),
    );

    Ok(address)
}

pub fn create_wallet(
    compose_path: String,
    container_name: String,
    datadir: String,
) -> Result<(), Error> {
    let datadir_flag = &format!("--datadir={}", datadir);

    let command = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        container_name.as_ref(),
        "bitcoin-cli",
        datadir_flag,
        "createwallet",
        container_name.as_ref(),
    ];
    info!(
        "container: {} command (create wallet): `docker-compose {}`",
        container_name,
        command.join(" ")
    );

    let output = process::Command::new("docker-compose")
        .args(command)
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("failed to create new address"));
    }
    Ok(())
}

pub fn create_address(
    compose_path: String,
    container_name: String,
    datadir: String,
) -> Result<String, Error> {
    let rpcwallet_flag = &format!("-rpcwallet={}", container_name);
    let datadir_flag = &format!("--datadir={}", datadir);

    let command = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        container_name.as_ref(),
        "bitcoin-cli",
        rpcwallet_flag,
        datadir_flag,
        "getnewaddress",
    ];
    info!(
        "container: {} command (create address): `docker-compose {}`",
        container_name,
        command.join(" ")
    );

    let mut output = process::Command::new("docker-compose")
        .args(command)
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("failed to create new address"));
    }
    // drop the newline character
    output.stdout.pop();
    let address = from_utf8(&output.stdout)?.to_owned();
    Ok(address)
}

pub fn mine_to_address(
    compose_path: String,
    container_name: String,
    datadir: String,
    num_blocks: i64,
    address: String,
) {
    let datadir_flag = &format!("--datadir={}", datadir);
    let block_arg = &num_blocks.to_string();
    let command = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        container_name.as_ref(),
        "bitcoin-cli",
        datadir_flag,
        "generatetoaddress",
        block_arg,
        &address,
    ];
    info!(
        "container: {} command (mine to address): `docker-compose {}`",
        container_name,
        command.join(" ")
    );

    let output = process::Command::new("docker-compose")
        .args(command)
        .output()
        .unwrap();
    if !output.status.success() {
        error!(
            "failed to mine to address: {} {}",
            from_utf8(&output.stdout).unwrap().to_owned(),
            from_utf8(&output.stderr).unwrap().to_owned()
        );
    }
}
