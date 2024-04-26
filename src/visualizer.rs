use anyhow::{Error, Result};
use std::{
    fs::File,
    io::{LineWriter, Write},
};

use crate::{get_absolute_path, Options};

pub fn create_ui_config_files(options: &Options, network: &str) -> Result<(), Error> {
    let config_absolute_path = get_absolute_path(&options.ui_config_path)?;
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
        let macaroon = format!("ACCESS_MACAROON_PATH={} \n", node.macaroon_path);
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

    node_config.flush()?;

    Ok(())
}
