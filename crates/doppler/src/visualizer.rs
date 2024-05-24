use anyhow::{Error, Result};
use doppler_core::Options;
use doppler_parser::get_absolute_path;
use std::{fs::File, io::{LineWriter, Write}};

use crate::{Cln, Eclair, Lnd};


pub fn create_ui_config_files(options: &Options, ui_config_path: &str, network: &str) -> Result<(), Error> {
    let config_absolute_path = get_absolute_path(ui_config_path)?;
    let node_config_file = File::create(config_absolute_path)?;
    let mut node_config = LineWriter::new(node_config_file);
    let lnd_nodes = options.lnd_nodes.clone();
    for node in lnd_nodes {
        let lnd = (node.as_any()).downcast_ref::<Lnd>().unwrap();
        let header = format!("[{}] \n", lnd.alias);
        let macaroon = format!("ADMIN_MACAROON_PATH={} \n", lnd.macaroon_path);
        let api_endpoint = format!("API_ENDPOINT={} \n", lnd.server_url);
        let tls_cert = format!("TLS_CERT_PATH={} \n", lnd.certificate_path);
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
        let coreln = (node.as_any()).downcast_ref::<Cln>().unwrap();
        let header = format!("[{}] \n", coreln.alias);
        let macaroon = format!("ACCESS_MACAROON_PATH={} \n", coreln.macaroon_path);
        let api_endpoint = format!("API_ENDPOINT={} \n", coreln.server_url);
        let network = format!("NETWORK={} \n", network);
        let node_type = format!("TYPE={} \n", "coreln");
        node_config.write_all(header.as_bytes())?;
        node_config.write_all(node_type.as_bytes())?;
        node_config.write_all(macaroon.as_bytes())?;
        node_config.write_all(api_endpoint.as_bytes())?;
        node_config.write_all(network.as_bytes())?;
    }

    for node in options.eclair_nodes.clone() {
        let eclair = (node.as_any()).downcast_ref::<Eclair>().unwrap();
        let header = format!("[{}] \n", eclair.alias);
        let password = format!("API_PASSWORD={} \n", eclair.api_password);
        let api_endpoint = format!("API_ENDPOINT={} \n", eclair.server_url);
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
