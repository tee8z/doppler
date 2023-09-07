use anyhow::{Error, Result};
use docker_compose_types::{
    AdvancedNetworkSettings, AdvancedNetworks, MapOrEmpty, Networks, Ports, Service, Volumes,
};
use indexmap::IndexMap;

use crate::{create_folder, get_absolute_path, Options, NETWORK};

#[derive(Debug, Clone)]
pub struct Visualizer {
    pub name: String,
    pub ip: String,
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
            let mut found_ip: Option<_> = None;
            if let Networks::Advanced(AdvancedNetworks(networks)) =
                service.1.as_ref().unwrap().networks.clone()
            {
                if let MapOrEmpty::Map(advance_setting) = networks.first().unwrap().1 {
                    found_ip = advance_setting.ipv4_address.clone();
                }
            }
            let ip = found_ip.unwrap();
            let data_dir = format!("/app/server/data/{}", operator_name);

            Visualizer {
                name: operator_name.to_string(),
                ip,
                data_dir,
                container_name: container_name.to_string(),
            }
        })
        .collect();

    options.utility_services = services;
    Ok(())
}

pub fn build_visualizer(options: &mut Options, _name: &str) -> Result<(), Error> {
    let ip = options.new_ipv4().to_string();
    let mut cur_network = IndexMap::new();
    cur_network.insert(
        NETWORK.to_string(),
        MapOrEmpty::Map(AdvancedNetworkSettings {
            ipv4_address: Some(ip.clone()),
            ..Default::default()
        }),
    );

    //Need to create these folders now so the permissions are correct on the volumes
    let local_path = get_absolute_path("data/visualizer")?;
    create_folder(local_path.to_str().unwrap())?;
    let auth_path = get_absolute_path("data/visualizer/auth")?;
    create_folder(auth_path.to_str().unwrap())?;
    let config_path = get_absolute_path("data/visualizer/config")?;
    create_folder(config_path.to_str().unwrap())?;

    let visualizer = Service {
        image: Some("litch/operator:latest".to_string()),
        container_name: Some("doppler-visualizer".to_string()),
        ports: Ports::Short(vec!["5100:5000".to_string()]),
        volumes: Volumes::Simple(vec![
            format!("{}/auth:/app/server/auth:rw", local_path.to_string_lossy()),
            format!(
                "{}/config:/app/server/config:rw",
                local_path.to_string_lossy()
            ),
        ]),
        networks: Networks::Advanced(AdvancedNetworks(cur_network)),
        ..Default::default()
    };
    options
        .services
        .insert("visualizer".to_string(), Some(visualizer));

    let operator_name = "visualizer";

    let data_dir = format!("/app/server/data/{}", operator_name);

    options.utility_services.push(Visualizer {
        name: operator_name.to_owned(),
        ip,
        data_dir: data_dir,
        container_name: "doppler-visualizer".to_owned(),
    });
    Ok(())
}
