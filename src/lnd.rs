use crate::{copy_file, get_absolute_path, mine_to_address, Bitcoind, Lnd, Options, NETWORK};
use anyhow::{anyhow, Error, Result};
use conf_parser::processer::{read_to_file_conf, FileConf, Section};
use docker_compose_types::{Networks, Ports, Service, Volumes, EnvFile};
use log::{debug, error, info, trace};
use serde_yaml::{from_slice, Value};
use std::{fs::File, process::Command, str::from_utf8};

const LND_IMAGE: &str = "polarlightning/lnd:0.16.2-beta";

pub fn build_lnd(options: &mut Options, name: &str, pair_name: &str) -> Result<()> {
    let mut lnd_conf = get_lnd_config(options, name, pair_name).unwrap();
    debug!("{} volume: {}", name, lnd_conf.path_vol);

    let rest_port = options.new_port();
    let grpc_port = options.new_port();
    let container_name = format!("doppler-{}", name);
    lnd_conf.container_name = Some(container_name.clone());
    lnd_conf.server_url = Some(format!("http://localhost:{}", grpc_port));
    let lnd = Service {
        image: Some(LND_IMAGE.to_string()),
        container_name: Some(container_name.clone()),
        ports: Ports::Short(vec![
            format!("{}:{}", rest_port, lnd_conf.rest_port),
            format!("{}:{}", grpc_port, lnd_conf.grpc_port),
        ]),
        env_file: Some(EnvFile::Simple(".env".to_owned())),
        volumes: Volumes::Simple(vec![format!("{}:/home/lnd/.lnd:rw", lnd_conf.path_vol)]),
        networks: Networks::Simple(vec![NETWORK.to_string()]),
        ..Default::default()
    };
    options.services.insert(container_name.clone(), Some(lnd));
    info!(
        "connect to {} via rest using port {} and via grpc using port {} with admin.macaroon found at {}",
        container_name,
        rest_port,
        grpc_port,
        lnd_conf.macaroon_path.clone().unwrap(),
    );

    options.lnds.push(lnd_conf);
    Ok(())
}

pub fn get_lnd_config(options: &mut Options, name: &str, pair_name: &str) -> Result<Lnd, Error> {
    if options.bitcoinds.is_empty() {
        return Err(anyhow!(
            "bitcoind nodes need to be defined before lnd nodes can be setup"
        ));
    }

    let original = get_absolute_path("config/lnd.conf")?;
    let destination_dir = &format!("data/{}/.lnd", name);
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
    if let Some(node) = found_node {
        bitcoind_node = node;
    }

    set_bitcoind_values(&mut conf, bitcoind_node)?;

    let _ = copy_file(&conf, &destination_dir.clone(), "lnd.conf")?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();

    Ok(Lnd {
        name: Some(name.to_owned()),
        alias: name.to_owned(),
        container_name: None,
        pubkey: None,
        server_url: None,
        certificate_path: Some(format!("{}/tls.crt", full_path)),
        macaroon_path: Some(format!(
            "{}/data/chain/bitcoin/{}/admin.macaroon",
            full_path, "regtest"
        )),
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

pub fn get_node_info(lnd: &mut Lnd, compose_path: String) -> Result<(), Error> {
    let command = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        lnd.container_name.as_ref().unwrap().as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        "getinfo",
    ];
    info!(
        "container: {} command (pubkey): `docker-compose {}`",
        lnd.container_name.clone().unwrap(),
        command.join(" ")
    );
    let output = Command::new("docker-compose").args(command).output()?;

    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        if let Some(pubkey) = response
            .as_mapping()
            .and_then(|obj| obj.get("identity_pubkey"))
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            lnd.pubkey = Some(pubkey);
        } else {
            error!("no pubkey found");
        }
    }
    trace!(
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(())
}

pub fn fund_node(lnd: &mut Lnd, miner: &Bitcoind, compose_path: String) -> Result<(), Error> {
    create_lnd_wallet(lnd, compose_path.clone())?;
    let address = create_lnd_address(lnd, compose_path.clone())?;
    mine_to_address(
        compose_path,
        miner.container_name.as_ref().unwrap().to_owned(),
        miner.data_dir.clone(),
        2,
        address,
    );
    Ok(())
}

pub fn create_lnd_wallet(lnd: &mut Lnd, compose_path: String) -> Result<(), Error> {
    let command = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        lnd.container_name.as_ref().unwrap().as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        "createwallet",
    ];
    info!(
        "container: {} command (createwallet): `docker-compose {}`",
        lnd.container_name.clone().unwrap(),
        command.join(" ")
    );
    let output = Command::new("docker-compose").args(command).output()?;

    if output.status.success() {
        let _response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
    }
    trace!(
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(())
}

pub fn create_lnd_address(lnd: &mut Lnd, compose_path: String) -> Result<String, Error> {
    let command = vec![
        "-f",
        compose_path.as_ref(),
        "exec",
        "--user",
        "1000:1000",
        lnd.container_name.as_ref().unwrap().as_ref(),
        "lncli",
        "--lnddir=/home/lnd/.lnd",
        "--network=regtest",
        "newaddress",
        "p2tr" //set as a taproot address, maybe make it
    ];
    info!(
        "container: {} command (newaddress): `docker-compose {}`",
        lnd.container_name.clone().unwrap(),
        command.join(" ")
    );
    let output = Command::new("docker-compose").args(command).output()?;
    let mut found_address: Option<String> = None;
    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        if let Some(address) = response
            .as_mapping()
            .and_then(|obj| obj.get("address"))
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            found_address = Some(address);
        } else {
            error!("no addess found");
        }
    }
    trace!(
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );

    Ok(found_address.unwrap())
}
