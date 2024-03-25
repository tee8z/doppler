use anyhow::{Error, Result};
use docker_compose_types::{DependsOnOptions, Networks, Ports, Service, Volumes};
use std::{
    fs::File,
    io::{LineWriter, Write},
};

use crate::{create_folder, get_absolute_path, Lnd, Options, NETWORK};

#[derive(Debug, Clone)]
pub struct Visualizer {
    pub name: String,
    pub data_dir: String,
    pub container_name: String,
}

pub fn add_visualizer(options: &mut Options) -> Result<(), Error> {
    let services: Vec<_> = options
        .services
        .iter_mut()
        .filter(|service| service.0.contains("visualizer"))
        .map(|service| {
            let container_name = service.0;
            let operator_name = container_name.split('-').last().unwrap();
            let data_dir = format!("/app/server/data/{}", operator_name);

            Visualizer {
                name: operator_name.to_string(),
                data_dir,
                container_name: container_name.to_string(),
            }
        })
        .collect();

    options.utility_services = services;
    Ok(())
}

pub fn build_visualizer(options: &mut Options, _name: &str) -> Result<(), Error> {
    let lnd_node = options
        .lnd_nodes
        .first()
        .expect("an lnd node must be configured before using a visualizer");

    //Need to create these folders now so the permissions are correct on the volumes
    let root_directory = get_absolute_path("data/visualizer")?;
    let auth_directory = get_absolute_path("data/visualizer/auth")?;
    let config_directory = get_absolute_path("data/visualizer/config")?;
    create_folder(root_directory.to_str().unwrap())?;
    create_folder(auth_directory.to_str().unwrap())?;
    create_folder(config_directory.to_str().unwrap())?;
    // Can only connect with lnd nodes at the moment with the visualizer
    for lnd in options.lnd_nodes.clone() {
        create_config_files(config_directory.to_str().unwrap(), &lnd)?;
    }
    let depends_on_options = DependsOnOptions::Simple(vec![lnd_node.container_name.clone()]);
    let visualizer = Service {
        image: Some("litch/operator:latest".to_string()),
        container_name: Some("doppler-visualizer".to_string()),
        ports: Ports::Short(vec!["5100:5000".to_string()]),
        depends_on: depends_on_options,
        volumes: Volumes::Simple(vec![
            format!("{}:/app/server/auth:rw", auth_directory.to_string_lossy()),
            format!(
                "{}:/app/server/config:rw",
                config_directory.to_string_lossy()
            ),
        ]),
        networks: Networks::Simple(vec![NETWORK.to_owned()]),
        ..Default::default()
    };
    options
        .services
        .insert("visualizer".to_string(), Some(visualizer));

    let operator_name = "visualizer";

    let data_dir = format!("/app/server/data/{}", operator_name);
    options.utility_services.push(Visualizer {
        name: operator_name.to_owned(),
        data_dir,
        container_name: "doppler-visualizer".to_owned(),
    });
    Ok(())
}

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