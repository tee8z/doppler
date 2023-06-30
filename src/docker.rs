use anyhow::Result;
use log::debug;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

const BITCOIND_IMAGE: &str = "polarlightning/bitcoind:25.0";
const LND_IMAGE: &str = "polarlightning/lnd:0.16.2-beta";

#[derive(Default, Debug)]
pub struct Options {
    pub network_name: Option<String>,
    pub bitcoind_lnd_pairs: Vec<NodePairing>,
}

#[derive(Default, Debug)]
pub struct NodePairing {
    pub bitcoind: Bitcoind,
    pub lnd: Option<Lnd>,
}

#[derive(Default, Debug)]
pub struct Bitcoind {
    pub name: String,
    pub network: String,
    pub rpchost: String,
    pub user: String,
    pub password: String,
    pub zmqpubrawblock: String,
    pub zmqpubrawtx: String,
}

#[derive(Default, Debug)]
pub struct Lnd {
    pub name: String,
    pub pubkey: String,
    pub alias: String,
    pub server_url: String,
    pub macaroon_path: String,
    pub certificate_path: String,
}

pub fn create_docker_network(options: &mut Options) -> Result<()> {
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

fn get_bitcoind_vol(ident: &str) -> Result<String, anyhow::Error> {
    let original = get_absolute_path("config/bitcoin.conf")?;
    let destination_dir = &format!("data/{}/.bitcoin", ident);
    let _ = copy_file(
        original.to_str().unwrap(),
        &destination_dir.clone(),
        "bitcoin.conf",
    )?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();
    Ok(full_path)
}

fn get_lnd_vol(ident: &str) -> Result<String, anyhow::Error> {
    let original = get_absolute_path("config/lnd.conf")?;
    let destination_dir = &format!("data/{}/.lnd", ident);
    let _ = copy_file(
        original.to_str().unwrap(),
        &destination_dir.clone(),
        "lnd.conf",
    )?;
    let full_path = get_absolute_path(destination_dir)?
        .to_str()
        .unwrap()
        .to_string();
    Ok(full_path)
}

fn get_absolute_path(relative_path: &str) -> Result<PathBuf, anyhow::Error> {
    let current_dir = std::env::current_dir()?;
    let absolute_path = current_dir.join(relative_path);

    Ok(absolute_path)
}

fn copy_file(
    source_file: &str,
    destination_directory: &str,
    conf_name: &str,
) -> Result<PathBuf, anyhow::Error> {
    let mut source: File = File::open(source_file)?;
    let destination_file = format!("{}/{}", destination_directory, conf_name);
    if Path::new(destination_directory).exists() {
        //TODO: figure out how to update conf file in directory between runs
        return get_absolute_path(&destination_file);
    }

    create_dir_all(destination_directory)?;
    let mut destination = File::create(destination_file.clone())?;
    let mut buffer = [0; 1024];
    loop {
        let n = source.read(&mut buffer)?;
        if n == 0 {
            break;
        }

        destination.write_all(&buffer[..n])?;
    }

    get_absolute_path(&destination_file)
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
    let bitcoind_vol = get_bitcoind_vol(ident).unwrap();
    debug!("{} bitcoind vol: {}", ident, bitcoind_vol);

    let output = Command::new("docker")
        .args([
            "run",
            "--detach",
            "--name",
            &format!("doppler-{}", ident),
            "--volume",
            &format!("{}:/home/bitcoin/.bitcoin:rw", bitcoind_vol),
            "--network",
            (options.network_name.as_ref().unwrap()),
            BITCOIND_IMAGE,
        ])
        .output()?;

    debug!(
        "output.stdout: {}, output.stderr: {}",
        str::from_utf8(&output.stdout)?,
        str::from_utf8(&output.stderr)?
    );
    Ok(())
}

pub fn start_lnd(options: &mut Options, ident: &str) -> Result<()> {
    let lnd_vol = get_lnd_vol(ident).unwrap();
    debug!("{} volume: {}", ident, lnd_vol);
    clear_old_image(ident).unwrap();
    let output = Command::new("docker")
        .args([
            "run",
            "--detach",
            "--name",
            &format!("doppler-{}", ident),
            "--volume",
            &format!("{}:/home/lnd/.lnd:rw", lnd_vol),
            "--network",
            (options.network_name.as_ref().unwrap()),
            LND_IMAGE,
        ])
        .output()?;

    debug!(
        "output.stdout: {}, output.stderr: {}",
        str::from_utf8(&output.stdout)?,
        str::from_utf8(&output.stderr)?
    );
    Ok(())
}
