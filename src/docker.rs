extern crate ini;
use crate::{
    get_absolute_path, pair_bitcoinds, L1Node, L2Node, NodeCommand, Options,
};
use anyhow::{anyhow, Error};
use ini::Ini;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use slog::{debug, error, info, Logger};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::Ipv4Addr;
use std::os::unix::prelude::PermissionsExt;
use std::process::{Command, Output};
use std::str::{from_utf8, FromStr};
use std::thread;
use std::time::Duration;

pub const NETWORK: &str = "doppler";
pub const SUBNET: &str = "10.5.0.0/16";

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
    options.load_visualizer()?;
    debug!(options.global_logger(), "loaded visualizer");
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
    debug!(options.global_logger(), "started cluster");
    //simple wait for docker-compose to spin up
    thread::sleep(Duration::from_secs(6));
    pair_bitcoinds(options)?;

    //TODO: make optional to be mining in the background
    start_miners(options)?;
    setup_l2_nodes(options)?;
    mine_initial_blocks(options)?;
    if !options.utility_services.is_empty() {
        provision_visualizer(options)?;
    }
    if options.aliases {
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
fn start_miners(options: &Options) -> Result<(), Error> {
    // kick of miners in background, mine every x interval
    options
        .bitcoinds
        .iter()
        .filter(|bitcoind| bitcoind.get_miner_time().is_some())
        .for_each(|bitcoind| bitcoind.clone().start_mining(options).unwrap());

    Ok(())
}

fn mine_initial_blocks(options: &Options) -> Result<(), Error> {
    // mine 100+ blocks
    let miner = options
        .bitcoinds
        .iter()
        .find(|bitcoinds| bitcoinds.get_container_name().contains("miner"));
    if miner.is_none() {
        return Err(anyhow!(
            "at least one miner is required to be setup for this cluster to run"
        ));
    }
    miner.unwrap().mine_bitcoin(options, 100)?;
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

    let logger = options.global_logger();
    options.get_l2_nodes().into_iter().for_each(|node| {
        let found_miner = miner.unwrap();
        match node.fund_node(&options.clone(), found_miner) {
            Ok(_) => info!(
                logger,
                "container: {} funded",
                node.get_container_name().clone()
            ),
            Err(e) => error!(logger, "failed to start/fund node: {}", e),
        }
    });

    connect_l2_nodes(options)?;

    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VisualizerNode {
    name: String,
    host: String,
    alias: String,
    pubkey: String,
}

#[derive(Serialize)]
struct VisualizerConfig {
    nodes: Vec<VisualizerNode>,
}

fn provision_visualizer(options: &Options) -> Result<(), Error> {
    debug!(options.global_logger(), "provisioning visualizer");
    let visualizer_config = build_visualizer_config(options)?;
    write_visualizer_config_to_ini(&visualizer_config)?;
    copy_authentication_files(options)?;
    Ok(())
}

fn build_visualizer_config(options: &Options) -> Result<VisualizerConfig, Error> {
    let mut config = VisualizerConfig { nodes: vec![] };
    options.lnd_nodes.iter().for_each(|node| {
        let name = node.get_name();
        let alias = node.get_alias();
        let pubkey = node.get_cached_pubkey();
        let host = node.rpc_server.clone();

        let visualizer_node = VisualizerNode {
            name: name.to_owned(),
            host,
            alias: alias.to_owned(),
            pubkey,
        };
        config.nodes.push(visualizer_node);
    });

    Ok(config)
}

fn write_visualizer_config_to_ini(config: &VisualizerConfig) -> Result<(), Error> {
    // Step 1: create an empty Ini object
    let mut conf = Ini::new();

    // Step 2: Loop through the nodes in VisualizerConfig and populate the Ini object
    for node in &config.nodes {
        // Creating a section for each VisualizerNode based on its name
        conf.with_section(Some(node.name.clone()))
            .set("host", &node.host)
            .set("alias", &node.alias)
            .set("pubkey", &node.pubkey);
    }

    // Step 3: Write Ini object to a file
    let path = get_absolute_path("data/visualizer/config/nodes.ini")?;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    conf.write_to(&mut file)?;


    let mut app_conf = Ini::new();
    app_conf
        .with_section(Some("logging".to_owned()))
        .set("log_dir", "./logs");

    let path = get_absolute_path("data/visualizer/config/graph_server.ini")?;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    app_conf.write_to(&mut file)?;
    Ok(())
}

fn copy_authentication_files(options: &Options) -> Result<(), Error> {
    debug!(
        options.global_logger(),
        "copying macaroon and tls cert files"
    );
    options.lnd_nodes.iter().for_each(|node| {
        let container_name = node.get_container_name();
        let name = container_name.split('-').last().unwrap();

        let dest_macaroon_path = format!("data/visualizer/auth/{}.macaroon", name);
        let dest_tls_cert_path = format!("data/visualizer/auth/{}.cert", name);
        let dest_macaroon_path = get_absolute_path(&dest_macaroon_path).unwrap();
        let dest_tls_cert_path = get_absolute_path(&dest_tls_cert_path).unwrap();
        let source_macaroon_path = node.macaroon_path.clone();
        let source_tls_cert_path = node.certificate_path.clone();
        let source_macaroon_path = get_absolute_path(&source_macaroon_path).unwrap();
        let source_tls_cert_path = get_absolute_path(&source_tls_cert_path).unwrap();
        debug!(
            options.global_logger(),
            "copying macaroon from {} to {}",
            source_macaroon_path.display(),
            dest_macaroon_path.display()
        );

        std::fs::copy(source_macaroon_path, dest_macaroon_path).unwrap();
        debug!(
            options.global_logger(),
            "copying tls cert from {} to {}",
            source_tls_cert_path.display(),
            dest_tls_cert_path.display()
        );
        std::fs::copy(source_tls_cert_path, dest_tls_cert_path).unwrap();
    });
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
    {docker_command} -f ./doppler-cluster.yaml exec --user 1000:1000 {container_name} lncli --lnddir=/home/lnd/.lnd --network=regtest --macaroonpath=/home/lnd/.lnd/data/chain/bitcoin/regtest/admin.macaroon --rpcserver={ip}:10000 "$@"
}}
"#,
        docker_command= docker_command,
        container_name= lnd.get_container_name(),
        name=name,
        ip =lnd.get_ip()));
        script_content.push('\n');
    });
    options.cln_nodes.iter().for_each(|lnd| {
        let name = lnd.get_container_name().split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec {container_name} lightning-cli --lightning-dir=/home/clightning --network=regtest "$@"
}}
"#,
        docker_command= docker_command,
        container_name= lnd.get_container_name(),
        name=name));
        script_content.push('\n');
    });
    options.eclair_nodes.iter().for_each(|lnd| {
        let name = lnd.get_container_name().split('-').last().unwrap();
        script_content.push_str(&format!(
            r#"
{name}() {{
    {docker_command} -f ./doppler-cluster.yaml exec --user 1000:1000 {container_name} eclair-cli -p test1234! "$@"
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
    let script_path = "scripts/container_aliases.sh";
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
            amt: None,
            subcommand: None,
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
