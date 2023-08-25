use crate::{
    connect, fund_node, get_absolute_path, get_bitcoinds, get_lnds, get_node_info, get_cln_node_info, mine_bitcoin,
    pair_bitcoinds, start_mining, Lnd, NodeCommand, Options,
};
use anyhow::{anyhow, Error};
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use slog::{debug, error, info, Logger};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::Ipv4Addr;
use std::os::unix::prelude::PermissionsExt;
use std::process::{Command, Output};
use std::str::{from_utf8, FromStr};
use std::thread;
use std::time::Duration;

pub const NETWORK: &str = "doppler";
pub const SUBNET: &str = "10.5.0.0/16";

pub fn load_options_from_compose(options: &mut Options, compose_path: &str) -> Result<(), Error> {
    options.compose_path = Some(compose_path.clone().to_owned());
    options.load_compose()?;
    debug!(
        options.global_logger(),
        "loaded {} file", options.docker_command
    );
    get_bitcoinds(options)?;
    debug!(options.global_logger(), "loaded bitcoinds");
    get_lnds(options)?;
    debug!(options.global_logger(), "loaded lnds");
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
    logger: &Logger,
    docker_command: String,
    command_name: String,
    commands: Vec<&str>,
) -> Result<Output, Error> {
    let commands = add_commands(docker_command.clone(), commands);

    info!(
        logger,
        "({}): {} {}",
        command_name,
        docker_command,
        commands.clone().join(" "),
    );
    let output = Command::new(docker_command.clone())
        .args(commands)
        .output()?;

    debug!(
        logger,
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
    debug!(options.global_logger(), "started cluster");
    //simple wait for docker-compose to spin up
    thread::sleep(Duration::from_secs(6));
    pair_bitcoinds(options)?;

    //TODO: make optional to be mining in the background
    start_miners(options)?;
    setup_lnd_nodes(options, options.global_logger())?;
    mine_initial_blocks(options)?;
    update_visualizer_conf(options)?;
    if options.aliases {
        update_bash_alias(options)?;
    }
    Ok(())
}

fn start_docker_compose(options: &mut Options) -> Result<(), Error> {
    let commands: Vec<&str> = vec![
        "-f",
        options.compose_path.as_ref().unwrap().as_ref(),
        "up",
        "-d",
    ];
    run_command(
        &options.global_logger(),
        options.docker_command.clone(),
        "start up".to_owned(),
        commands,
    )?;
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
        options.docker_command.clone(),
        compose_path.to_owned(),
        miner_container,
        miner_data_dir,
        100,
    )?;
    Ok(())
}

fn setup_lnd_nodes(options: &mut Options, logger: Logger) -> Result<(), Error> {
    let compose_path = options.compose_path.as_ref().unwrap();
    let docker_command = options.docker_command.clone();
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
        let result = get_node_info(
            docker_command.clone(),
            &logger,
            node,
            compose_path_clone.clone(),
        )
        .and_then(|_| {
            info!(
                logger,
                "container: {} pubkey: {}",
                node.container_name.clone(),
                node.pubkey.clone().unwrap()
            );
            fund_node(
                options.docker_command.clone(),
                &logger,
                node,
                &miner.unwrap().clone(),
                compose_path_clone.clone(),
            )
        });

        match result {
            Ok(_) => info!(logger, "container: {} funded", node.container_name.clone()),
            Err(e) => error!(logger, "failed to start/fund node: {}", e),
        }
    });

    connect_lnd_nodes(options)?;

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
            macaroon: admin_macaroon,
        };
        config.nodes.push(visualizer_node);
    });
    let config_json = get_absolute_path("visualizer/config.json")?;
    debug!(options.global_logger(),"config_json: {}", config_json.display());
    let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(config_json)?;

    let json_string = to_string(&config)?;
    debug!(options.global_logger(),"preping json config file");
    file.write_all(json_string.as_bytes())?;
    debug!(options.global_logger(),"saved json config file");

    Ok(())
}

fn update_bash_alias(options: &mut Options) -> Result<(), Error> {
    let docker_command = if options.docker_command.contains('-') {
        options.docker_command.to_owned()
    } else {
        "docker compose".to_owned()
    };
    let mut script_content = String::new();
    script_content.push_str(&format!("{}", options.shell_type.unwrap_or_default()));
    options.lnds.iter().for_each(|lnd| {
        let name = lnd.container_name.split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec --user 1000:1000 {container_name} lncli --lnddir=/home/lnd/.lnd --network=regtest --macaroonpath=/home/lnd/.lnd/data/chain/bitcoin/regtest/admin.macaroon --rpcserver={ip}:10000 "$@"
}}
"#,
        docker_command= docker_command,
        container_name= lnd.container_name,
        name=name,
        ip =lnd.ip));
        script_content.push('\n');
    });
    options.bitcoinds.iter().for_each(|bitcoind| {
        let name = bitcoind.container_name.split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec --user 1000:1000 {container_name} bitcoin-cli "$@"
}}
"#,
            docker_command= docker_command,
            name = name,
            container_name = bitcoind.container_name,
        ));
        script_content.push('\n');
    });
    let script_path = "scripts/container_aliases.sh";
    let full_path = get_absolute_path(script_path)?;
    let mut file: File = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(full_path.clone())?;
    file.write_all(script_content.as_bytes())?;
    debug!(options.global_logger(), "wrote aliases script @ {}", full_path.display());
    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o755);
    file.set_permissions(permissions)?;
    debug!(options.global_logger(), "wrote aliases script");

    Ok(())
}

fn get_admin_macaroon(node: &mut Lnd) -> Result<String, Error> {
    let macaroon_path = node.macaroon_path.clone();
    let mut file = OpenOptions::new().read(true).open(macaroon_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let mac_as_hex = hex::encode(buffer);
    Ok(mac_as_hex)
}

pub fn generate_ipv4_sequence_in_subnet(
    logger: Logger,
    subnet: &str,
    current_ip: &Ipv4Addr,
) -> Ipv4Addr {
    let cidr = IpNetwork::from_str(subnet).unwrap();
    let end_ip = match cidr {
        IpNetwork::V4(cidr_v4) => cidr_v4.broadcast(),
        _ => panic!("Only IPv4 is supported"),
    };
    let mut next_ip = *current_ip;

    next_ip = Ipv4Addr::from(u32::from(next_ip) + 1);
    if next_ip > end_ip {
        error!(logger, "went over the last ip in ranges!")
    }
    next_ip
}

fn connect_lnd_nodes(options: &mut Options) -> Result<(), Error> {
    let mut get_a_node = options.lnds.iter();
    options.lnds.iter().for_each(|from_lnd| {
        let back_a_node = get_a_node.next_back();
        if back_a_node.is_none() {
            return;
        }
        let mut to_lnd = back_a_node.unwrap();
        if to_lnd.name == from_lnd.name {
            to_lnd = get_a_node.next().unwrap();
        }
        let node_command = &NodeCommand {
            name: "connect".to_owned(),
            from: from_lnd.name.clone(),
            to: to_lnd.name.clone(),
            amt: None,
            subcommand: None,
        };
        connect(options, node_command).unwrap_or_default();
    });
    Ok(())
}
