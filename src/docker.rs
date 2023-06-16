use anyhow::Result;
use log::debug;
use std::process::Command;
use std::str;

const BITCOIND_IMAGE: &str = "polarlightning/bitcoind:25.0";

#[derive(Default, Debug)]
pub struct Options {
    pub network_name: Option<String>,
}

pub fn create_docker_network(options: &mut Options) -> Result<()> {
    let output = Command::new("docker")
        .args([
            "network",
            "create",
            "--driver",
            "bridge",
            &options.network_name.as_ref().unwrap(),
        ])
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
    let output = Command::new("docker")
        .args([
            "run",
            "--detach",
            "--name",
            &format!("doppler-{}", ident),
            "--network",
            &options.network_name.as_ref().unwrap(),
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
