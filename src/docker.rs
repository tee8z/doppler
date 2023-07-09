use crate::conf_handler::{get_bitcoind_config, get_lnd_config, Options};

use anyhow::Result;
use log::{debug, info};
use std::process::Command;
use std::str;

const BITCOIND_IMAGE: &str = "polarlightning/bitcoind:25.0";
const LND_IMAGE: &str = "polarlightning/lnd:0.16.2-beta";

pub fn create_docker_network(options: &mut Options) -> Result<()> {
    if check_if_network_exists(options)? {
        return Ok(());
    }
    let output = Command::new("docker")
        .args([
            "network",
            "create",
            "--driver",
            "bridge",
            (options.network_name.as_ref().unwrap()),
        ])
        .output()?;
    debug!(
        "output.stdout: {}, output.stderr: {}",
        str::from_utf8(&output.stdout)?,
        str::from_utf8(&output.stderr)?
    );
    Ok(())
}

fn check_if_network_exists(options: &mut Options) -> Result<bool, anyhow::Error> {
    let output = Command::new("docker")
        .args([
            "network",
            "inspect",
            (options.network_name.as_ref().unwrap()),
        ])
        .output()?;
    let err_output = str::from_utf8(&output.stderr)?;
    Ok(!err_output.contains("No such network"))
}

fn clear_old_image(ident: &str) -> Result<()> {
    let output = Command::new("docker")
        .args(["rm", "--force", "--volumes", &format!("doppler-{}", ident)])
        .output()?;
    debug!(
        "output.stdout: {}, output.stderr: {}",
        str::from_utf8(&output.stdout)?,
        str::from_utf8(&output.stderr)?
    );
    Ok(())
}

pub fn start_bitcoind(options: &mut Options, ident: &str) -> Result<()> {
    create_docker_network(options)?;
    clear_old_image(ident).unwrap();
    let mut bitcoind_conf = get_bitcoind_config(options, ident).unwrap();
    debug!("{} bitcoind vol: {}", ident, bitcoind_conf.path_vol);
    let rpc_port = options.new_port();
    let container_name = format!("doppler-{}", ident);
    let output = Command::new("docker")
        .args([
            "run",
            "--detach",
            "--name",
            &container_name,
            "--volume",
            &format!("{}:/home/bitcoin/.bitcoin:rw", bitcoind_conf.path_vol),
            "--network",
            (options.network_name.as_ref().unwrap()),
            "-p",
            &format!("{}:{}", rpc_port, bitcoind_conf.rpcport),
            BITCOIND_IMAGE,
        ])
        .output()?;
    bitcoind_conf.name = Some(container_name.clone());
    debug!(
        "output.stdout: {}, output.stderr: {}",
        str::from_utf8(&output.stdout)?,
        str::from_utf8(&output.stderr)?
    );
    info!(
        "connect to {} via rpc using port {} with username {} and password {}",
        container_name, rpc_port, bitcoind_conf.user, bitcoind_conf.password
    );
    options.bitcoinds.push(bitcoind_conf);

    Ok(())
}

pub fn start_lnd(options: &mut Options, ident: &str) -> Result<()> {
    let mut lnd_conf = get_lnd_config(options, ident).unwrap();
    debug!("{} volume: {}", ident, lnd_conf.path_vol);
    clear_old_image(ident).unwrap();
    let rest_port = options.new_port();
    let grpc_port = options.new_port();
    let container_name = format!("doppler-{}", ident);
    let output = Command::new("docker")
        .args([
            "run",
            "--detach",
            "--name",
            &container_name,
            "--volume",
            &format!("{}:/home/lnd/.lnd:rw", lnd_conf.path_vol),
            "--network",
            (options.network_name.as_ref().unwrap()),
            "-p",
            &format!("{}:{}", rest_port, lnd_conf.rest_port),
            "-p",
            &format!("{}:{}", grpc_port, lnd_conf.grpc_port),
            LND_IMAGE,
        ])
        .output()?;

    debug!(
        "output.stdout: {}, output.stderr: {}",
        str::from_utf8(&output.stdout)?,
        str::from_utf8(&output.stderr)?
    );
    info!(
        "connect to {} via rest using port {} and via grpc using port {}", // with admin.macaroon found at {}",
        container_name,
        rest_port,
        grpc_port, //lnd_conf.macaroon_path.clone().unwrap(),
    );
    lnd_conf.name = Some(container_name);
    options.lnds.push(lnd_conf);
    Ok(())
}
