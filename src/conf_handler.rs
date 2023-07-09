use anyhow::{anyhow, Error};
use conf_parser::processer::{FileConf, Section};
use std::{
    fs::{create_dir_all, File},
    path::{Path, PathBuf},
    vec,
};

#[derive(Default, Debug)]
pub struct Options {
    pub network_name: Option<String>,
    pub bitcoinds: Vec<Bitcoind>,
    pub lnds: Vec<Lnd>,
    ports: Vec<i64>,
}

impl Options {
    pub fn new() -> Self {
        let starting_port = vec![9089];
        Self {
            network_name: None,
            bitcoinds: vec::Vec::new(),
            lnds: vec::Vec::new(),
            ports: starting_port,
        }
    }
    pub fn new_port(&mut self) -> i64 {
        let last_port = self.ports.last().unwrap();
        let next_port = last_port + 1;
        self.ports.push(next_port);
        next_port
    }
}

#[derive(Default, Debug)]
pub struct Bitcoind {
    pub name: Option<String>,
    pub rpchost: String,
    pub rpcport: String,
    pub user: String,
    pub password: String,
    pub zmqpubrawblock: String,
    pub zmqpubrawtx: String,
    pub path_vol: String,
}

#[derive(Default, Debug)]
pub struct Lnd {
    pub name: Option<String>,
    pub pubkey: Option<String>,
    pub alias: String,
    pub rest_port: String,
    pub grpc_port: String,
    pub server_url: Option<String>,
    pub macaroon_path: Option<String>,
    pub certificate_path: Option<String>,
    pub path_vol: String,
}

pub fn get_bitcoind_config(options: &mut Options, ident: &str) -> Result<Bitcoind, Error> {
    let original = get_absolute_path("config/bitcoin.conf")?;
    let destination_dir = &format!("data/{}/.bitcoin", ident);
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
        name: None,
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

pub fn get_lnd_config(options: &mut Options, ident: &str) -> Result<Lnd, Error> {
    if options.bitcoinds.is_empty() {
        return Err(anyhow!(
            "bitcoind nodes need to be defined before lnd nodes can be setup"
        ));
    }

    let original = get_absolute_path("config/lnd.conf")?;
    let destination_dir = &format!("data/{}/.lnd", ident);
    let source: File = File::open(original)?;

    let mut conf = conf_parser::processer::read_to_file_conf(&source)?;

    //TODO: determine which bitcoind to use, just using the first one for now
    let bitcoind_node = options.bitcoinds.first().unwrap();
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
            bitcoind_node.name.as_ref().unwrap(),
            &bitcoind_node.zmqpubrawblock
        )
        .as_str(),
    );
    bitcoind.set_property(
        "bitcoind.zmqpubrawtx",
        format!(
            "tcp://{}:{}",
            bitcoind_node.name.as_ref().unwrap(),
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
            bitcoind_node.name.as_ref().unwrap(),
            &bitcoind_node.rpchost
        )
        .as_str(),
    );

    Ok(())
}

fn get_absolute_path(relative_path: &str) -> Result<PathBuf, Error> {
    let current_dir = std::env::current_dir()?;
    let absolute_path = current_dir.join(relative_path);

    Ok(absolute_path)
}

fn copy_file(
    source_conf: &FileConf,
    destination_directory: &str,
    conf_name: &str,
) -> Result<PathBuf, anyhow::Error> {
    let destination_file = format!("{}/{}", destination_directory, conf_name);
    if Path::new(destination_directory).exists() {
        //TODO: figure out how to update conf file in directory between runs
        return get_absolute_path(&destination_file);
    }

    create_dir_all(destination_directory)?;
    conf_parser::processer::write_to_file(source_conf, &destination_file)?;

    get_absolute_path(&destination_file)
}
