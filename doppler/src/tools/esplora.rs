use crate::{create_folder, get_absolute_path, Bitcoind, Options, NETWORK};
use anyhow::{anyhow, Result};
use docker_compose_types::{
    DependsCondition, DependsOnOptions, Entrypoint, Environment, Networks, Ports, Service, Volumes,
};
use indexmap::IndexMap;
use std::{path::Path, process::Command};

use super::ToolImageInfo;

#[derive(Clone)]
pub struct Esplora {
    pub name: String,
    pub http_connection: String,
    pub electrum_port: String,
}

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
    let elects_port = options.new_port();

    let bitcoind: &Bitcoind = match options.get_bitcoind_by_name(target_node) {
        Ok(bitcoind) => bitcoind,
        Err(err) => return Err(err),
    };
    let esplora_container_name = format!("doppler-{}-{}", name, bitcoind.name);
    let mut conditional = IndexMap::new();
    conditional.insert(
        bitcoind.container_name.to_owned(),
        DependsCondition {
            condition: String::from("service_healthy"),
        },
    );
    let volume = &format!("data/{}/logs", name);
    create_folder(volume)?;
    let log_paths = [
        format!("{}/electrs/debug.log", volume),
        format!("{}/nginx/access.log", volume),
        format!("{}/nginx/error.log", volume),
        format!("{}/nginx/current", volume),
        format!("{}/prerenderer/current", volume),
        format!("{}/websocket/current", volume),
    ];
    for log_path in &log_paths {
        if let Some(parent) = Path::new(log_path).parent() {
            create_folder(parent.to_str().unwrap())?;
        }
        std::fs::File::create(log_path)?;
    }
    let full_path = get_absolute_path(volume)?.to_str().unwrap().to_string();

    let (uid, gid) = get_user_ids();

    let esplora = Service {
        image: Some(image.get_tag()),
        container_name: Some(esplora_container_name.clone()),
        depends_on: DependsOnOptions::Conditional(conditional),
        ports: Ports::Short(vec![
            format!("{}:50001", electrum_port), // Electrum RPC
            format!("{}:80", esplora_web_port), // Esplora Web Interface And API Server Port
            format!("{}:3000", elects_port),    // Esplora Web Interface And API Server Port
        ]),
        volumes: Volumes::Simple(vec![
            format!(
                "{}/electrs/debug.log:/data/logs/electrs/debug.log:rw",
                full_path
            ),
            format!("{}/nginx/error.log:/var/log/nginx/error.log:rw", full_path),
            format!(
                "{}/nginx/access.log:/var/log/nginx/access.log:rw",
                full_path
            ),
            format!("{}/nginx/current:/data/logs/nginx/current:rw", full_path),
            format!(
                "{}/prerenderer/current:/data/logs/prerenderer/current:rw",
                full_path
            ),
            format!(
                "{}/websocket/current:/data/logs/websocket/current:rw",
                full_path
            ),
        ]),
        environment: Environment::List(vec![
            format!("FLAVOR=bitcoin-{}", options.network),
            String::from("MODE=explorer"),
            String::from("DEBUG=verbose"),
            format!("FLAVOR=bitcoin-{}", options.network),
            format!("NETWORK={}", options.network),
            format!("DAEMON_RPC_ADDR={}", bitcoind.container_name),
            format!("DAEMON_RPC_PORT={}", bitcoind.rpcport),
            format!("RPC_USER={}", bitcoind.user),
            format!("RPC_PASS={}", bitcoind.password),
            format!("USER_ID={}", uid),
            format!("GROUP_ID={}", gid),
            String::from("STATIC_ROOT=http://localhost:5000/"),
        ]),
        networks: Networks::Simple(vec![NETWORK.to_owned()]),
        entrypoint: Some(Entrypoint::List(vec![
            "bash".to_owned(),
            "-c".to_owned(),
            format!(
                r#"chown $$USER_ID:$$GROUP_ID /data && \
cat > /srv/explorer/custom_run.sh << 'EOL'
{}
EOL
chmod +x /srv/explorer/custom_run.sh && \
exec /srv/explorer/custom_run.sh "$@""#,
                CUSTOM_RUN_SCRIPT
            ),
        ])),
        ..Default::default()
    };

    options
        .services
        .insert(esplora_container_name.clone(), Some(esplora));

    options.esplora.push(Esplora {
        name: name.to_owned(),
        http_connection: format!("http://localhost:{}", esplora_web_port),
        electrum_port: format!("localhost:{}", electrum_port),
    });

    Ok(())
}

fn get_user_ids() -> (String, String) {
    #[cfg(target_os = "macos")]
    {
        // On macOS staff group is typically 20
        let default_gid = "20".to_string();
        let uid = Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or("501".to_string()) // macOS typically starts user IDs at 501
            .trim()
            .to_string();

        (uid, default_gid)
    }

    #[cfg(target_os = "linux")]
    {
        let uid = Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or("1000".to_string())
            .trim()
            .to_string();

        let gid = Command::new("id")
            .arg("-g")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or("1000".to_string())
            .trim()
            .to_string();

        (uid, gid)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        ("1000".to_string(), "1000".to_string())
    }
}

// This custom script allows us to point esplora at an existing bitcoind instance instead of having it create one while it starts up \
// (which fixes many locking issues of two bitcoind instance trying to access the same data)
const CUSTOM_RUN_SCRIPT: &str = r#"#!/bin/bash
set -eo pipefail

echo "FLAVOR: $$FLAVOR"
echo "MODE: $$MODE"
echo "DEBUG: $$DEBUG"

# Validate required environment variables
if [ -z "$$NETWORK" ] || [ -z "$$DAEMON_RPC_ADDR" ] || [ -z "$$DAEMON_RPC_PORT" ] || [ -z "$$RPC_USER" ] || [ -z "$$RPC_PASS" ]; then
    echo "Required environment variables are not set"
    echo "NETWORK: $$NETWORK"
    echo "DAEMON_RPC_ADDR: $$DAEMON_RPC_ADDR"
    echo "DAEMON_RPC_PORT: $$DAEMON_RPC_PORT"
    echo "RPC_USER: $$RPC_USER"
    echo "RPC_PASS: $$RPC_PASS"
    exit 1
fi

# Validate flavor is regtest or signet
if [ "$$NETWORK" != "regtest" ] && [ "$$NETWORK" != "signet" ]; then
    echo "Only regtest and signet are supported"
    echo "For example: run.sh bitcoin-regtest explorer"
    exit 1
fi

STATIC_DIR=/srv/explorer/static/bitcoin-$${NETWORK}
if [ ! -d "$$STATIC_DIR" ]; then
    echo "Static directory $$STATIC_DIR not found"
    exit 1
fi

echo "Enabled mode $${MODE} for bitcoin-$${NETWORK}"

# Set up all required directories
mkdir -p /data/logs/{electrs,nginx,prerenderer,websocket}
mkdir -p /data/electrs_db/$$NETWORK
mkdir -p /etc/service/{nginx,prerenderer,websocket}/log
mkdir -p /etc/run_once
mkdir -p /var/run/electrs
mkdir -p /etc/service/socat

cp /srv/explorer/source/contrib/runits/socat.runit /etc/service/socat/run

# Configure nginx
NGINX_PATH="$${NETWORK}/"
NGINX_NOSLASH_PATH="$${NETWORK}"
NGINX_REWRITE="rewrite ^/$${NETWORK}(/.*)$$ \$$1 break;"
NGINX_REWRITE_NOJS="rewrite ^/$${NETWORK}(/.*)$$ \"/$${NETWORK}/nojs\$$1?\" permanent"
NGINX_CSP="default-src 'self'; script-src 'self' 'unsafe-eval'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'"
NGINX_LOGGING="access_log /var/log/nginx/access.log"

# Configure electrs
ELECTRS_DB_DIR="/data/electrs_db/$$NETWORK"
ELECTRS_LOG_FILE="/data/logs/electrs/debug.log"

# Start electrs in the background
if [ "$${DEBUG}" == "verbose" ]; then
    RUST_BACKTRACE=full /srv/explorer/electrs_bitcoin/bin/electrs \
        --timestamp \
        --http-addr 127.0.0.1:3000 \
        --network $$NETWORK \
        --daemon-rpc-addr "$${DAEMON_RPC_ADDR}:$${DAEMON_RPC_PORT}" \
        --cookie="$${RPC_USER}:$${RPC_PASS}" \
        --monitoring-addr 0.0.0.0:4224 \
        --electrum-rpc-addr 0.0.0.0:50001 \
        --db-dir "$$ELECTRS_DB_DIR" \
        --http-socket-file /var/electrs-rest.sock \
        --jsonrpc-import \
        --address-search \
        -vvvv > "$$ELECTRS_LOG_FILE" 2>&1 &
else
    /srv/explorer/electrs_bitcoin/bin/electrs \
        --timestamp \
        --http-addr 127.0.0.1:3000 \
        --network $$NETWORK \
        --daemon-rpc-addr "$${DAEMON_RPC_ADDR}:$${DAEMON_RPC_PORT}" \
        --cookie="$${RPC_USER}:$${RPC_PASS}" \
        --monitoring-addr 0.0.0.0:4224 \
        --electrum-rpc-addr 0.0.0.0:50001 \
        --db-dir "$$ELECTRS_DB_DIR" \
        --http-socket-file /var/electrs-rest.sock \
        --address-search \
        -vvvv > "$$ELECTRS_LOG_FILE" 2>&1 &
fi

ELECTRS_PID=$!

# Configure services
cp /srv/explorer/source/contrib/runits/nginx.runit /etc/service/nginx/run
cp /srv/explorer/source/contrib/runits/nginx-log.runit /etc/service/nginx/log/run
cp /srv/explorer/source/contrib/runits/nginx-log-config.runit /data/logs/nginx/config

cp /srv/explorer/source/contrib/runits/prerenderer.runit /etc/service/prerenderer/run
cp /srv/explorer/source/contrib/runits/prerenderer-log.runit /etc/service/prerenderer/log/run
cp /srv/explorer/source/contrib/runits/prerenderer-log-config.runit /data/logs/prerenderer/config

cp /srv/explorer/source/contrib/runits/websocket.runit /etc/service/websocket/run
cp /srv/explorer/source/contrib/runits/websocket-log.runit /etc/service/websocket/log/run
cp /srv/explorer/source/contrib/runits/websocket-log-config.runit /data/logs/websocket/config

chmod +x /etc/service/*/run

# Process nginx configuration
function preprocess(){
   in_file=$$1
   out_file=$$2
   cat $$in_file | \
   sed -e "s|{DAEMON}|bitcoin|g" \
       -e "s|{DAEMON_DIR}|$$DAEMON_DIR|g" \
       -e "s|{NETWORK}|$$NETWORK|g" \
       -e "s|{STATIC_DIR}|$$STATIC_DIR|g" \
       -e "s#{ELECTRS_ARGS}#$$ELECTRS_ARGS#g" \
       -e "s|{ELECTRS_BACKTRACE}|$$ELECTRS_BACKTRACE|g" \
       -e "s|{NGINX_LOGGING}|$$NGINX_LOGGING|g" \
       -e "s|{NGINX_PATH}|$$NGINX_PATH|g" \
       -e "s|{NGINX_CSP}|$$NGINX_CSP|g" \
       -e "s|{NGINX_REWRITE}|$$NGINX_REWRITE|g" \
       -e "s|{NGINX_REWRITE_NOJS}|$$NGINX_REWRITE_NOJS|g" \
       -e "s|{FLAVOR}|bitcoin-$$NETWORK|g" \
       -e "s|{NGINX_NOSLASH_PATH}|$$NGINX_NOSLASH_PATH|g" \
   >$$out_file
}

preprocess /srv/explorer/source/contrib/nginx.conf.in /etc/nginx/sites-enabled/default
sed -i 's/user www-data;/user root;/' /etc/nginx/nginx.conf

echo "Testing nginx configuration..."
nginx -t

# Start runit services
exec /srv/explorer/source/contrib/runit_boot.sh"#;
