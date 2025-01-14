use anyhow::{Error, Result};
use std::{
    fs::{self, File},
    io::{LineWriter, Write},
    path::Path,
};

use crate::{get_absolute_path, Options};

pub fn create_ui_config_files(options: &Options, network: &str) -> Result<(), Error> {
    let config_absolute_path = get_absolute_path(&options.ui_config_path)?;
    if let Some(parent) = Path::new(&config_absolute_path).parent() {
        fs::create_dir_all(parent)?;
    }
    let node_config_file = File::create(config_absolute_path)?;
    let mut node_config = LineWriter::new(node_config_file);
    for node in options.lnd_nodes.clone() {
        let header = format!("[{}] \n", node.alias);
        let macaroon = format!("ADMIN_MACAROON_PATH={} \n", node.macaroon_path);
        let api_endpoint = format!("API_ENDPOINT={} \n", node.server_url);
        let tls_cert = format!("TLS_CERT_PATH={} \n", node.certificate_path);
        let network = format!("NETWORK={} \n", network);
        let node_type = format!("TYPE={} \n", "lnd");
        node_config.write_all(header.as_bytes())?;
        node_config.write_all(node_type.as_bytes())?;
        node_config.write_all(macaroon.as_bytes())?;
        node_config.write_all(api_endpoint.as_bytes())?;
        node_config.write_all(tls_cert.as_bytes())?;
        node_config.write_all(network.as_bytes())?;
    }

    for node in options.cln_nodes.clone() {
        let header = format!("[{}] \n", node.alias);
        let macaroon = format!("RUNE={} \n", node.rune.unwrap_or_default());
        let api_endpoint = format!("API_ENDPOINT={} \n", node.server_url);
        let network = format!("NETWORK={} \n", network);
        let node_type = format!("TYPE={} \n", "coreln");
        node_config.write_all(header.as_bytes())?;
        node_config.write_all(node_type.as_bytes())?;
        node_config.write_all(macaroon.as_bytes())?;
        node_config.write_all(api_endpoint.as_bytes())?;
        node_config.write_all(network.as_bytes())?;
    }

    for node in options.eclair_nodes.clone() {
        let header = format!("[{}] \n", node.alias);
        let password = format!("API_PASSWORD={} \n", node.api_password);
        let api_endpoint = format!("API_ENDPOINT={} \n", node.server_url);
        let network = format!("NETWORK={} \n", network);
        let node_type = format!("TYPE={} \n", "eclair");
        node_config.write_all(header.as_bytes())?;
        node_config.write_all(node_type.as_bytes())?;
        node_config.write_all(password.as_bytes())?;
        node_config.write_all(api_endpoint.as_bytes())?;
        node_config.write_all(network.as_bytes())?;
    }

    for node in &options.bitcoinds {
        let header = format!("[{}] \n", node.name);
        let password = format!("PASSWORD={} \n", node.password);
        let user = format!("USER={} \n", node.user);
        let network = format!("NETWORK={} \n", network);
        let node_type = format!("TYPE={} \n", "bitcoind");
        let public_p2p = format!("P2P=localhost:{} \n", node.public_p2p.unwrap());
        let public_rpc = format!("RPC=localhost:{} \n", node.public_rpc.unwrap());
        node_config.write_all(header.as_bytes())?;
        node_config.write_all(node_type.as_bytes())?;
        node_config.write_all(password.as_bytes())?;
        node_config.write_all(user.as_bytes())?;
        node_config.write_all(public_p2p.as_bytes())?;
        node_config.write_all(public_rpc.as_bytes())?;
        node_config.write_all(network.as_bytes())?;
    }

    for node in &options.esplora {
        let header = format!("[{}] \n", node.name);
        let api_endpoint = format!("API_ENDPOINT={} \n", node.http_connection);
        let electrum_port = format!("ELECTRUM_PORT={} \n", node.electrum_port);
        let network = format!("NETWORK={} \n", network);
        let node_type = format!("TYPE={} \n", "esplora");
        node_config.write_all(header.as_bytes())?;
        node_config.write_all(node_type.as_bytes())?;
        node_config.write_all(api_endpoint.as_bytes())?;
        node_config.write_all(electrum_port.as_bytes())?;
        node_config.write_all(network.as_bytes())?;
    }

    node_config.flush()?;

    Ok(())
}
