use crate::{copy_file, get_absolute_path, Bitcoind, Options, NETWORK};
use anyhow::{Error, Result};
use conf_parser::processer::{FileConf, Section};
use docker_compose_types::{Networks, Service, Ports, Volumes};
use log::debug;
use std::fs::File;

const BITCOIND_IMAGE: &str = "polarlightning/bitcoind:25.0";

pub fn build_bitcoind(options: &mut Options, name: &str) -> Result<()> {
    let mut bitcoind_conf = get_bitcoind_config(options, name).unwrap();
    debug!("{} bitcoind vol: {}", name, bitcoind_conf.path_vol);

    let rpc_port = options.new_port();
    let container_name = format!("doppler-{}", name);

    let bitcoind = Service {
        image: Some(BITCOIND_IMAGE.to_string()),
        container_name: Some(container_name.clone()),
        ports: Ports::Short(vec![format!("{}:{}", rpc_port, bitcoind_conf.rpcport)]),
        volumes: Volumes::Simple(vec![format!(
            "{}:/home/bitcoin/.bitcoin:rw",
            bitcoind_conf.path_vol
        )]),
        networks: Networks::Simple(vec![NETWORK.to_string()]),
        ..Default::default()
    };
    options.services.insert(container_name.clone(), Some(bitcoind));
    bitcoind_conf.container_name = Some(container_name.clone());
    options.bitcoinds.push(bitcoind_conf);

    Ok(())
}

pub fn get_bitcoind_config(options: &mut Options, name: &str) -> Result<Bitcoind, Error> {
    let original = get_absolute_path("config/bitcoin.conf")?;
    let destination_dir = &format!("data/{}/.bitcoin", name);
    let source: File = File::open(original)?;
    let conf = conf_parser::processer::read_to_file_conf(&source)?;
    let mut mut_conf = conf;
    let mut_ref = &mut mut_conf;
    set_regtest_section(mut_ref, options)?;

    let _ = copy_file(&mut_conf, &destination_dir.clone(), "bitcoin.conf")?;

    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();

    let regtest_section = mut_conf.sections.get("regtest").unwrap();

    Ok(Bitcoind {
        name: Some(name.to_string()),
        container_name: None,
        path_vol: full_path,
        user: regtest_section.clone().get_property("rpcuser"),
        password: regtest_section.clone().get_property("rpcpassword"),
        rpchost: regtest_section.clone().get_property("rpcport"),
        rpcport: regtest_section.clone().get_property("rpcport"),
        zmqpubrawblock: regtest_section
            .clone()
            .get_property("zmqpubrawblock")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
        zmqpubrawtx: regtest_section
            .clone()
            .get_property("zmqpubrawtx")
            .split(':')
            .last()
            .unwrap()
            .to_owned(),
    })
}

fn set_regtest_section(conf: &mut FileConf, options: &mut Options) -> Result<(), Error> {
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

    Ok(())
}
