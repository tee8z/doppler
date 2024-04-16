extern crate ini;
use crate::{
    create_ui_config_files, get_absolute_path, pair_bitcoinds, L1Node, L2Node, NodeCommand, Options,
};
use anyhow::{anyhow, Error};
use slog::{debug, error, info};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::prelude::PermissionsExt;
use std::process::{Command, Output};
use std::str::from_utf8;
use std::thread;
use std::time::Duration;

pub const NETWORK: &str = "doppler";

pub fn load_options_from_external_nodes(
    options: &mut Options,
    external_nodes_folder_path: &str,
) -> Result<(), Error> {
    //Skips any docker setup/calls, using external nodes instead
    options.load_external_nodes(external_nodes_folder_path)?;
    debug!(
        options.global_logger(),
        "loaded {} file", external_nodes_folder_path
    );
    options.load_lnds()?;
    debug!(options.global_logger(), "loaded lnds");
    let network = options.external_nodes.clone().unwrap()[0].network.clone();
    create_ui_config_files(&options, &network)?;
    Ok(())
}

pub fn load_options_from_compose(options: &mut Options, compose_path: &str) -> Result<(), Error> {
    options.compose_path = Some(compose_path.to_owned());
    options.load_compose()?;
    debug!(
        options.global_logger(),
        "loaded {} file", options.docker_command
    );
    options.load_bitcoinds()?;
    debug!(options.global_logger(), "loaded bitcoinds");
    options.load_lnds()?;
    debug!(options.global_logger(), "loaded lnds");
    options.load_eclairs()?;
    debug!(options.global_logger(), "loaded eclairs");
    options.load_coreln()?;
    debug!(options.global_logger(), "loaded corelsn");
    Ok(())
}

pub fn add_commands(docker_command: String, additional_commands: Vec<&str>) -> Vec<&str> {
    let mut commands = vec![];
    if !docker_command.contains('-') {
        commands.push("compose");
    }

    commands.extend(additional_commands);
    commands
}

pub fn run_command(
    options: &Options,
    command_name: String,
    commands: Vec<&str>,
) -> Result<Output, Error> {
    let commands = add_commands(options.docker_command.clone(), commands);

    info!(
        options.global_logger(),
        "({}): {} {}",
        command_name,
        options.docker_command,
        commands.clone().join(" "),
    );
    let output = Command::new(options.docker_command.clone())
        .args(commands)
        .output()?;

    debug!(
        options.global_logger(),
        "output.stdout: {}, output.stderr: {}",
        from_utf8(&output.stdout)?,
        from_utf8(&output.stderr)?
    );
    Ok(output)
}

pub fn run_cluster(options: &mut Options, compose_path: &str) -> Result<(), Error> {
    options.compose_path = Some(compose_path.to_owned());

    options
        .save_compose(options.docker_command.clone(), compose_path)
        .map_err(|err| {
            anyhow!(
                "Failed to save {} file @ {}: {}",
                options.docker_command,
                compose_path,
                err
            )
        })?;
    debug!(options.global_logger(), "saved cluster config");

    start_docker_compose(options)?;
    create_ui_config_files(options, &options.network)?;

    debug!(options.global_logger(), "started cluster");
    //simple wait for docker-compose to spin up
    thread::sleep(Duration::from_secs(6));
    pair_bitcoinds(options)?;
    if options.network == "regtest" {
        mine_initial_blocks(options)?;
    }
    setup_l2_nodes(options)?;
    if options.aliases && options.external_nodes.is_none() {
        update_bash_alias(options)?;
    }

    Ok(())
}

fn start_docker_compose(options: &Options) -> Result<(), Error> {
    let commands: Vec<&str> = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "up",
        "-d",
    ];
    run_command(options, "start up".to_owned(), commands)?;
    Ok(())
}

fn mine_initial_blocks(options: &Options) -> Result<(), Error> {
    // mine 200+ blocks
    let miner = options
        .bitcoinds
        .iter()
        .find(|bitcoinds| bitcoinds.get_container_name().contains("miner"));
    if miner.is_none() {
        return Err(anyhow!(
            "at least one miner is required to be setup for this cluster to run"
        ));
    }
    miner.unwrap().mine_bitcoin(options, 200)?;
    Ok(())
}

fn setup_l2_nodes(options: &mut Options) -> Result<(), Error> {
    options.add_pubkeys_l2_nodes()?;

    let miner = options
        .bitcoinds
        .iter()
        .find(|bitcoinds| bitcoinds.get_container_name().contains("miner"));
    if miner.is_none() {
        return Err(anyhow!(
            "at least one miner is required to be setup for this cluster to run"
        ));
    }

    //TODO: add faucet call here instead of miner for signet
    if options.network == "regtest" {
        connect_l2_nodes(options)?;

        let logger = options.global_logger();
        options.get_l2_nodes().into_iter().for_each(|node| {
            let found_miner = miner.unwrap();
            match node.fund_node(&options.clone(), found_miner) {
                Ok(_) => info!(logger, "container: {} funded", node.get_container_name()),
                Err(e) => error!(logger, "failed to start/fund node: {}", e),
            }
        });
    }
    Ok(())
}

fn update_bash_alias(options: &Options) -> Result<(), Error> {
    let docker_command = if options.docker_command.contains('-') {
        options.docker_command.to_owned()
    } else {
        "docker compose".to_owned()
    };
    let mut script_content = String::new();
    script_content.push_str(&format!("{}", options.shell_type.unwrap_or_default()));
    options.lnd_nodes.iter().for_each(|lnd| {
        let name = lnd.get_container_name().split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec --user 1000:1000 {container_name} lncli --lnddir=/home/lnd/.lnd --network={network} --macaroonpath=/home/lnd/.lnd/data/chain/bitcoin/{network}/admin.macaroon --rpcserver=localhost:10000 "$@"
}}
"#,
        docker_command= docker_command,
        container_name= lnd.get_container_name(),
        name=name,
        network=options.network));
        script_content.push('\n');
    });
    options.cln_nodes.iter().for_each(|lnd| {
        let name = lnd.get_container_name().split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec {container_name} lightning-cli --lightning-dir=/home/clightning --network={network} "$@"
}}
"#,
        docker_command= docker_command,
        container_name= lnd.get_container_name(),
        name=name,
    network=options.network));
        script_content.push('\n');
    });
    options.eclair_nodes.iter().for_each(|lnd| {
        let name = lnd.get_container_name().split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec --user 1000:1000 {container_name} eclair-cli -p test1234 "$@"
}}
"#,
        docker_command= docker_command,
        container_name= lnd.get_container_name(),
        name=name));
        script_content.push('\n');
    });
    options.bitcoinds.iter().for_each(|bitcoind| {
        let container_name = bitcoind.get_container_name();
        let name = container_name.split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec --user 1000:1000 {container_name} bitcoin-cli "$@"
}}
"#,
            docker_command= docker_command,
            name = name,
            container_name = bitcoind.get_container_name(),
        ));
        script_content.push('\n');
    });
    let script_path = "scripts/aliases.sh";
    let full_path = get_absolute_path(script_path)?;
    let mut file: File = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(full_path.clone())?;
    file.write_all(script_content.as_bytes())?;
    debug!(
        options.global_logger(),
        "wrote aliases script @ {}",
        full_path.display()
    );
    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o755);
    file.set_permissions(permissions)?;
    debug!(options.global_logger(), "wrote aliases script");

    Ok(())
}

pub fn update_bash_alias_external(options: &Options) -> Result<(), Error> {
    let mut script_content = String::new();
    script_content.push_str(&format!("{}", options.shell_type.unwrap_or_default()));
    options.external_nodes.clone().unwrap().iter().for_each(|lnd| {
        script_content.push_str(&format!(
            r#"
{name}() {{
     lncli --network={network} --macaroonpath={macaroon_path} --rpcserver={rpcserver} --tlscertpath="" "$@"
}}
"#, 
        name=lnd.node_alias, network=lnd.network, macaroon_path=lnd.macaroon_path, rpcserver=lnd.api_endpoint));
        script_content.push('\n');
    });
    let script_path = "scripts/aliases.sh";
    let full_path = get_absolute_path(script_path)?;
    let mut file: File = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(full_path.clone())?;
    file.write_all(script_content.as_bytes())?;

    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o755);
    file.set_permissions(permissions)?;
    debug!(
        options.global_logger(),
        "wrote aliases script @ {}",
        full_path.display()
    );

    Ok(())
}

fn connect_l2_nodes(options: &Options) -> Result<(), Error> {
    let mut get_a_node = options.lnd_nodes.iter();
    options.get_l2_nodes().iter().for_each(|from_node| {
        let back_a_node = get_a_node.next_back();
        if back_a_node.is_none() {
            return;
        }
        let mut to_node = back_a_node.unwrap();
        if to_node.get_name() == from_node.get_name() {
            let next_node = get_a_node.next();
            if let Some(next_node) = next_node {
                to_node = next_node;
            } else {
                return;
            }
        }
        let node_command = &NodeCommand {
            name: "connect".to_owned(),
            from: from_node.get_name().to_owned(),
            to: to_node.get_name().to_owned(),
            ..Default::default()
        };
        from_node.connect(options, node_command).unwrap_or_default();
    });
    Ok(())
}

pub fn restart_service(options: &Options, service_name: String) -> Result<Output, Error> {
    let compose_path = options.compose_path.as_ref().unwrap();

    let commands = vec!["-f", compose_path, "restart", &service_name];
    let output = run_command(options, "restart service".to_owned(), commands.clone())?;
    Ok(output)
}
