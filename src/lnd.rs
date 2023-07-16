use crate::{copy_file, get_absolute_path, Bitcoind, Lnd, Options, NETWORK};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{Networks, Ports, Service, Volumes};
use log::{debug, info};
use std::fs::File;

const LND_IMAGE: &str = "polarlightning/lnd:0.16.2-beta";

pub fn build_lnd(options: &mut Options, ident: &str, pair_name: &str) -> Result<()> {
    let mut lnd_conf = get_lnd_config(options, ident, pair_name).unwrap();
    debug!("{} volume: {}", ident, lnd_conf.path_vol);

    let rest_port = options.new_port();
    let grpc_port = options.new_port();
    let container_name = format!("doppler-{}", ident);

    let lnd = Service {
        image: Some(LND_IMAGE.to_string()),
        container_name: Some(container_name.clone()),
        ports: Ports::Short(vec![
            format!("{}:{}", rest_port, lnd_conf.rest_port),
            format!("{}:{}", grpc_port, lnd_conf.grpc_port),
        ]),
        volumes: Volumes::Simple(vec![format!("{}:/home/lnd/.lnd:rw", lnd_conf.path_vol)]),
        networks: Networks::Simple(vec![NETWORK.to_string()]),
        ..Default::default()
    };
    options.services.insert(container_name.clone(), Some(lnd));
    info!(
        "connect to {} via rest using port {} and via grpc using port {}", // with admin.macaroon found at {}",
        container_name,
        rest_port,
        grpc_port, //lnd_conf.macaroon_path.clone().unwrap(),
    );

    lnd_conf.name = Some(container_name);
    options.lnds.push(lnd_conf);
    Ok(())
}

pub fn get_lnd_config(options: &mut Options, ident: &str, pair_name: &str) -> Result<Lnd, Error> {
    if options.bitcoinds.is_empty() {
        return Err(anyhow!(
            "bitcoind nodes need to be defined before lnd nodes can be setup"
        ));
    }

    let original = get_absolute_path("config/lnd.conf")?;
    let destination_dir = &format!("data/{}/.lnd", ident);
    let source: File = File::open(original)?;

    let mut conf = read_to_file_conf(&source)?;
    let mut bitcoind_node = options
        .bitcoinds
        .first()
        .expect("a layer 1 needs to be confirgured before using a layer 2 node");
    let found_node = options.bitcoinds.iter().find(|&bitcoind| {
        bitcoind
            .name
            .as_ref()
            .unwrap()
            .eq_ignore_ascii_case(pair_name)
    });
    if found_node.is_some() {
        bitcoind_node = found_node.unwrap();
    }

    set_bitcoind_values(&mut conf, bitcoind_node)?;

    let _ = copy_file(&conf, &destination_dir.clone(), "lnd.conf")?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();

    let application_section = conf.sections.get("Application Options").unwrap();

    Ok(Lnd {
        name: None,
        pubkey: None, //can't get until it starts
        server_url: None,
        certificate_path: None,
        alias: application_section.clone().get_property("alias"),
        macaroon_path: None,
        path_vol: full_path,
        grpc_port: "10009".to_owned(),
        rest_port: "8080".to_owned(),
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
            bitcoind_node.container_name.as_ref().unwrap(),
            &bitcoind_node.zmqpubrawblock
        )
        .as_str(),
    );
    bitcoind.set_property(
        "bitcoind.zmqpubrawtx",
        format!(
            "tcp://{}:{}",
            bitcoind_node.container_name.as_ref().unwrap(),
            &bitcoind_node.zmqpubrawtx
        )
        .as_str(),
    );
    bitcoind.set_property("bitcoind.rpcpass", &bitcoind_node.password);
    bitcoind.set_property("bitcoind.rpcuser", &bitcoind_node.user);
    bitcoind.set_property(
        "bitcoind.rpchost",
        format!(
            "{}:{}",
            bitcoind_node.container_name.as_ref().unwrap(),
            &bitcoind_node.rpchost
        )
        .as_str(),
    );

    Ok(())
}
