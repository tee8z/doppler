use anyhow::{Error, Result};
use std::{
    fs::File,
    io::{LineWriter, Write},
};

use crate::{create_folder, get_absolute_path, Lnd, Options, NETWORK};

//TODO: have this build the ./ui/config/info.conf file
fn create_config_files(config_folder_path: &str, lnd: &Lnd) -> Result<(), Error> {
    let node_config_file = File::create(format!("{config_folder_path}/nodes.ini"))?;
    let mut node_config = LineWriter::new(node_config_file);

    let header = format!("[{}] \n", lnd.alias);
    let node_connection = format!("host={} \n", lnd.rpc_server);
    node_config.write_all(header.as_bytes())?;
    node_config.write_all(node_connection.as_bytes())?;

    node_config.flush()?;

    Ok(())
}
