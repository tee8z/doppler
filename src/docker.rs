use std::thread;
use std::time::Duration;
use std::{process::Command, str::from_utf8};

use crate::{fund_node, get_node_info, pair_bitcoinds, start_mining};
use crate::{mine_bitcoin, Options};
use anyhow::{anyhow, Error};
use log::debug;
use log::error;
use log::info;

pub const NETWORK: &str = "doppler";

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
    thread::sleep(Duration::from_secs(5));

    //TODO: make optional to be mining in the background
    start_miners(options)?;
    setup_nodes(options)?;
    mine_initial_blocks(options)?;
  
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
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(())
}
fn start_miners(options: &mut Options) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();

    // kick of miners in background, mine every x interval
    options
        .bitcoinds
        .iter()
        .filter(|bitcoind| bitcoind.miner_time.is_some())
        .for_each(|bitcoind| start_mining(options.kill_signal.clone(), bitcoind, &compose_path.clone()).unwrap());

    Ok(())
}

fn mine_initial_blocks(options: &mut Options) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    // mine 100+ blocks
    let miner = options
        .bitcoinds
        .iter()
        .find(|bitcoinds| bitcoinds.container_name.as_ref().unwrap().contains("miner"));
    if miner.is_none() {
        return Err(anyhow!(
            "at least one miner is required to be setup for this cluster to run"
        ));
    }
    let miner_container = miner.unwrap().container_name.clone();
    let miner_data_dir = miner.unwrap().data_dir.clone();
    mine_bitcoin(
        compose_path.to_owned(),
        miner_container.unwrap(),
        miner_data_dir,
        100,
    )?;
    Ok(())
}

fn setup_nodes(options: &mut Options) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let miner = options
        .bitcoinds
        .iter()
        .find(|bitcoinds| bitcoinds.container_name.as_ref().unwrap().contains("miner"));
    if miner.is_none() {
        return Err(anyhow!(
            "at least one miner is required to be setup for this cluster to run"
        ));
    }
    options.lnds.iter_mut().for_each(|node| {
        let result = get_node_info(node, compose_path.clone()).and_then(|_| {
            info!(
                "container: {} pubkey: {}",
                node.container_name.clone().unwrap(),
                node.pubkey.clone().unwrap()
            );
            fund_node(node, miner.unwrap().clone(), compose_path.clone())
        });

        match result {
            Ok(_) => info!(
                "container: {} funded",
                node.container_name.clone().unwrap_or_default()
            ),
            Err(e) => error!("failed to start/fund node: {}", e),
        }
    });
    Ok(())
}
