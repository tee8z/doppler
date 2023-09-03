use anyhow::{Error, Result};
use docker_compose_types::{AdvancedNetworkSettings, MapOrEmpty, Service, Ports, Volumes, Networks, AdvancedNetworks};
use indexmap::IndexMap;

use crate::{get_absolute_path, Options, NETWORK};

const OPERATOR_IMAGE: &str =  "litch/operator:latest";

// pub fn load_operator(&mut self) -> Result<(), Error> {
//     let operator = Service {
//         image: Some(OPERATOR_IMAGE.to_string()),
//         container_name: Some("doppler-operator".to_string()),
//         ports: Ports::Short(vec!["5100:5000".to_string()]),
//         volumes: Volumes::Simple(vec![
//             "/Users/litch/code/doppler/data/operator/auth:/app/server/auth".to_string(),
//             "/Users/litch/code/doppler/data/operator/config:/app/server/config".to_string(),
//         ]),
//         networks: Networks::Advanced(AdvancedNetworks(cur_network)),
//         ..Default::default()
//     };
// }

pub fn add_operator(options: &mut Options) -> Result<(), Error> {
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

    options.utility_services.push(operator);
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