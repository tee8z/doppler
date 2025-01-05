use crate::{Bitcoind, Options, NETWORK};
use anyhow::{anyhow, Result};
use docker_compose_types::{
    Command, DependsOnOptions, Environment, Networks, Ports, Service, Volumes,
};

use super::ToolImageInfo;

pub fn build_esplora(
    options: &mut Options,
    name: &str,
    image: &ToolImageInfo,
    target_node: &str,
) -> Result<()> {
    if options.bitcoinds.is_empty() {
        return Err(anyhow!(
            "bitcoind nodes need to be defined before esplora nodes can be setup"
        ));
    }
    let electrum_port = options.new_port();
    let esplora_web_port = options.new_port();

    let bitcoind: &Bitcoind = match options.get_bitcoind_by_name(target_node) {
        Ok(bitcoind) => bitcoind,
        Err(err) => return Err(err),
    };
    let esplora_container_name = format!("doppler-{}-{}", name, bitcoind.name);
    let mut env_vars = vec![
        String::from("DEBUG=verbose"),
        format!("BITCOIN_NODE_HOST={}", bitcoind.container_name),
        format!("BITCOIN_NODE_PORT={}", bitcoind.rpcport),
    ];
    if options.network == "regtest" {
        env_vars.push(String::from("NO_REGTEST_MINING=1"))
    }

    let esplora = Service {
        image: Some(image.get_tag()),
        container_name: Some(esplora_container_name.clone()),
        depends_on: DependsOnOptions::Simple(vec![bitcoind.container_name.clone()]),
        ports: Ports::Short(vec![
            format!("{}:50001", electrum_port), // Electrum RPC
            format!("{}:80", esplora_web_port), // Esplora Web Interface And API Server Port
        ]),
        volumes: Volumes::Simple(vec![format!("{}:/data:rw", bitcoind.path_vol)]),
        command: Some(Command::Args(vec![
            "/srv/explorer/run.sh".to_owned(),
            format!("bitcoin-{}", options.network),
            "explorer".to_owned(),
        ])),
        networks: Networks::Simple(vec![NETWORK.to_owned()]),
        environment: Environment::List(env_vars.into()),
        ..Default::default()
    };

    options
        .services
        .insert(esplora_container_name.clone(), Some(esplora));

    Ok(())
}
