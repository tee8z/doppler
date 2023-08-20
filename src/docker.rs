use crate::{
    fund_node, get_absolute_path, get_bitcoinds, get_lnds, get_node_info, mine_bitcoin,
    pair_bitcoinds, start_mining, Lnd, Options,
};
use anyhow::{anyhow, Error};
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use slog::{debug, error, info, Logger};
use std::fs::File;
use std::io::Read;
use std::net::Ipv4Addr;
use std::process::Command;
use std::str::{from_utf8, FromStr};
use std::thread;
use std::time::Duration;

pub const NETWORK: &str = "doppler";
pub const SUBNET: &str = "10.5.0.0/16";

pub fn load_options_from_compose(options: &mut Options, compose_path: &str) -> Result<(), Error> {
    options.compose_path = Some(compose_path.clone().to_owned());
    options.load_compose()?;
    debug!(options.global_logger(), "loaded docker-compose file");
    get_bitcoinds(options)?;
    debug!(options.global_logger(), "loaded bitcoinds");
    get_lnds(options)?;
    debug!(options.global_logger(), "loaded lnds");
    Ok(())
}

pub fn run_cluster(options: &mut Options, compose_path: &str) -> Result<(), Error> {
    options.compose_path = Some(compose_path.to_owned());

    pair_bitcoinds(options)?;

    options.save_compose(compose_path).map_err(|err| {
        anyhow!(
            "Failed to save docker-compose file @ {}: {}",
            compose_path,
            err
        )
    })?;
    start_docker_compose(options)?;

    //simple wait for docker-compose to spin up
    thread::sleep(Duration::from_secs(6));

    //TODO: make optional to be mining in the background
    start_miners(options)?;
    setup_nodes(options, options.global_logger())?;
    mine_initial_blocks(options)?;
    update_visualizer_conf(options)?;
    Ok(())
}

fn start_docker_compose(options: &mut Options) -> Result<(), Error> {
    let output = Command::new("docker-compose")
        .args([
            "-f",
            options.compose_path.as_ref().unwrap().as_ref(),
            "up",
            "-d",
        ])
        .output()?;
    debug!(
        options.global_logger(),
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(())
}
fn start_miners(options: &mut Options) -> Result<(), Error> {
    // kick of miners in background, mine every x interval
    options
        .bitcoinds
        .iter()
        .filter(|bitcoind| bitcoind.miner_time.is_some())
        .for_each(|bitcoind| start_mining(options.clone(), bitcoind).unwrap());

    Ok(())
}

fn mine_initial_blocks(options: &mut Options) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    // mine 100+ blocks
    let miner = options
        .bitcoinds
        .iter()
        .find(|bitcoinds| bitcoinds.container_name.contains("miner"));
    if miner.is_none() {
        return Err(anyhow!(
            "at least one miner is required to be setup for this cluster to run"
        ));
    }
    let miner_container = miner.unwrap().container_name.clone();
    let miner_data_dir = miner.unwrap().data_dir.clone();
    let logger = &options.global_logger();
    mine_bitcoin(
        logger,
        compose_path.to_owned(),
        miner_container,
        miner_data_dir,
        100,
    )?;
    Ok(())
}

fn setup_nodes(options: &mut Options, logger: Logger) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let miner = options
        .bitcoinds
        .iter()
        .find(|bitcoinds| bitcoinds.container_name.contains("miner"));
    if miner.is_none() {
        return Err(anyhow!(
            "at least one miner is required to be setup for this cluster to run"
        ));
    }

    options.lnds.iter_mut().for_each(|node| {
        let compose_path_clone = compose_path.clone();
        let result = get_node_info(&logger, node, compose_path_clone.clone()).and_then(|_| {
            info!(
                logger,
                "container: {} pubkey: {}",
                node.container_name.clone(),
                node.pubkey.clone().unwrap()
            );
            fund_node(
                &logger,
                node,
                &miner.unwrap().clone(),
                compose_path_clone.clone(),
            )
        });

        match result {
            Ok(_) => info!(
                logger,
                "container: {} funded",
                node.container_name.clone()
            ),
            Err(e) => error!(logger, "failed to start/fund node: {}", e),
        }
    });
    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VisualizerConfig {
    nodes: Vec<VisualizerNode>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VisualizerNode {
    name: String,
    host: String,
    macaroon: String,
}

fn update_visualizer_conf(options: &mut Options) -> Result<(), Error> {
    let mut config = VisualizerConfig { nodes: vec![] };
    options.lnds.iter_mut().for_each(|node| {
        let name = node.name.clone();
        let admin_macaroon = get_admin_macaroon(node).unwrap();
        let visualizer_node = VisualizerNode {
            name,
            host: node.server_url.clone(),
            macaroon: admin_macaroon
        };
        config.nodes.push(visualizer_node);
    });
    let json_string = to_string(&config)?;
    let config_json = get_absolute_path("./visualizer/config.json")?;
    std::fs::write(config_json, json_string)?;

    Ok(())
}

fn get_admin_macaroon(node: &mut Lnd) -> Result<String, Error> {
    let macaroon_path = node.macaroon_path.clone();
    let mut file = File::open(macaroon_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let mac_as_hex = hex::encode(buffer);
    Ok(mac_as_hex)
}

pub fn generate_ipv4_sequence_in_subnet(logger: Logger, subnet: &str, current_ip: &Ipv4Addr) -> Ipv4Addr {
    let cidr = IpNetwork::from_str(subnet).unwrap();
    let end_ip = match cidr {
        IpNetwork::V4(cidr_v4) => cidr_v4.broadcast(),
        _ => panic!("Only IPv4 is supported"),
    };
    let mut next_ip = current_ip.clone();

    next_ip = Ipv4Addr::from(u32::from(next_ip) + 1);
    if next_ip > end_ip {
        error!(logger, "went over the last ip in ranges!")
    }
    next_ip
}
