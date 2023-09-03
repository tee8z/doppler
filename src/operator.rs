use anyhow::{Error, Result};
use docker_compose_types::{AdvancedNetworkSettings, MapOrEmpty, Service, Ports, Volumes, Networks, AdvancedNetworks};
use indexmap::IndexMap;

use crate::{get_absolute_path, Options, NETWORK};

const OPERATOR_IMAGE: &str = "litch/operator:latest";

#[derive(Debug, Clone)]
pub struct Operator {
    pub name: String,
    pub ip: String,
    pub data_dir: String,
    pub container_name: String,
}

pub fn add_operator(options: &mut Options) -> Result<(), Error> {
    let operators: Vec<_> = options
        .services
        .iter_mut()
        .filter(|service| service.0.contains("operator"))
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

            Operator {
                name: operator_name.to_string(),
                ip: ip.to_string(),
                data_dir: data_dir.clone(),
                container_name: container_name.to_string(),
            }
        })
        .collect();

    options.utility_services = operators;
    Ok(())
}

pub fn build_operator(options: &mut Options, _name: &str) -> Result<(), Error> {
    let ip = options.new_ipv4().to_string();
    let mut cur_network = IndexMap::new();
    cur_network.insert(
        NETWORK.to_string(),
        MapOrEmpty::Map(AdvancedNetworkSettings {
            ipv4_address: Some(ip),
            ..Default::default()
        }),
    );

    let local_path = get_absolute_path("data/operator")?
        .to_str()
        .unwrap()
        .to_string();

    let operator = Service {
        image: Some(OPERATOR_IMAGE.to_string()),
        container_name: Some("doppler-operator".to_string()),
        ports: Ports::Short(vec!["5100:5000".to_string()]),
        volumes: Volumes::Simple(vec![
            format!("{}/auth:/app/server/auth", local_path),
            format!("{}/config:/app/server/config", local_path),
        ]),
        networks: Networks::Advanced(AdvancedNetworks(cur_network)),
        ..Default::default()
    };
    options
        .services
        .insert("operator".to_string(), Some(operator));
    // options.utility_services.push(operator);
    Ok(())
}
