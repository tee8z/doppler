use anyhow::{Error, Result};
use std::{
    fs::File,
    io::{LineWriter, Write},
};

use crate::{get_absolute_path, Lnd};
/*
TODO:
- make the configuration and UI handle more than just lnd
*/
pub fn create_config_files(
    config_folder_path: &str,
    network: &str,
    lnds: Vec<Lnd>,
) -> Result<(), Error> {
    let config_absolute_path = get_absolute_path(config_folder_path)?;
    let node_config_file = File::create(config_absolute_path)?;
    let mut node_config = LineWriter::new(node_config_file);
    for node in lnds {
        let header = format!("[{}] \n", node.alias);
        let macaroon = format!("ADMIN_MACAROON_PATH={} \n", node.macaroon_path);
        let api_endpoint = format!("API_ENDPOINT={} \n", node.server_url);
        let tls_cert = format!("TLS_CERT_PATH={} \n", node.certificate_path);
        let network = format!("NETWORK={} \n", network);
        node_config.write_all(header.as_bytes())?;
        node_config.write_all(macaroon.as_bytes())?;
        node_config.write_all(api_endpoint.as_bytes())?;
        node_config.write_all(tls_cert.as_bytes())?;
        node_config.write_all(network.as_bytes())?;
    }

    node_config.flush()?;

    Ok(())
}
