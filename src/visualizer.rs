use anyhow::{Error, Result};
use docker_compose_types::{DependsOnOptions, Networks, Ports, Service, Volumes};

use crate::{create_folder, get_absolute_path, Options, NETWORK};

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
    let local_path = get_absolute_path("data/visualizer")?;
    create_folder(local_path.to_str().unwrap())?;
    let auth_path = get_absolute_path("data/visualizer/auth")?;
    create_folder(auth_path.to_str().unwrap())?;
    let config_path = get_absolute_path("data/visualizer/config")?;
    create_folder(config_path.to_str().unwrap())?;

    let depends_on_options = DependsOnOptions::Simple(vec![lnd_node.container_name.clone()]);

    let visualizer = Service {
        image: Some("litch/operator:latest".to_string()),
        container_name: Some("doppler-visualizer".to_string()),
        ports: Ports::Short(vec!["5100:5000".to_string()]),
        depends_on: depends_on_options,
        volumes: Volumes::Simple(vec![
            format!("{}/auth:/app/server/auth:rw", local_path.to_string_lossy()),
            format!(
                "{}/config:/app/server/config:rw",
                local_path.to_string_lossy()
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
        data_dir: data_dir,
        container_name: "doppler-visualizer".to_owned(),
    });
    Ok(())
}
