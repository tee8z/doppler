use std::{process::Command, str::from_utf8};

use anyhow::{anyhow, Error, Result};
use log::debug;

use crate::Options;

pub const NETWORK: &str = "doppler";

pub fn run_cluster(options: &mut Options, compose_path: &str) -> Result<(), Error> {

    options.save_compose(compose_path).map_err(|err| {
        anyhow!(
            "Failed to save docker-compose file @ {}: {}",
            compose_path,
            err
        )
    })?;
    start_docker_compose(options, compose_path)?;
    Ok(())
}

fn start_docker_compose(options: &mut Options, compose_path: &str) -> Result<(), Error> {
    let output = Command::new("docker-compose")
        .args(["-f", compose_path, "up", "-d"])
        .output()?;
    debug!(
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(())
}

pub fn clear_docker() -> Result<()> {
    Ok(())
}
