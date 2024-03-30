use crate::{run_command, L2Node, Lnd, NodeCommand, Options};
use anyhow::{anyhow, Error, Result};
use slog::{debug, error, info};
use std::{fs::OpenOptions, io::Read, str::from_utf8, thread, time::Duration};

#[derive(Default, Debug, Clone)]
pub struct LndCli;

impl LndCli {
    pub fn get_node_pubkey(&self, lnd: &Lnd, options: &Options) -> Result<String, Error> {
        let rpc_command = lnd.get_rpc_server_command();
        let macaroon_path = lnd.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();
        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            &lnd.container_name,
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "getinfo",
        ];

        let mut retries = 3;
        let mut output_found = None;
        while retries > 0 {
            let output = run_command(options, "pubkey".to_owned(), commands.clone())?;
            if from_utf8(&output.stderr)?.contains(
            "the RPC server is in the process of starting up, but not yet ready to accept calls",
        ) {
            debug!(options.global_logger(), "trying to get pubkey again");
            thread::sleep(Duration::from_secs(2));
            retries -= 1;
        } else {
            output_found = Some(output);
            break;
        }
        }
        if let Some(output) = output_found {
            if output.status.success() {
                if let Some(pubkey) = lnd.get_property("identity_pubkey", output) {
                    return Ok(pubkey);
                } else {
                    error!(options.global_logger(), "no pubkey found");
                }
            }
        }
        Ok("".to_owned())
    }

    pub fn create_lnd_address(&self, lnd: &Lnd, options: &Options) -> Result<String, Error> {
        let rpc_command = lnd.get_rpc_server_command();
        let macaroon_path = lnd.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            &lnd.container_name,
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "newaddress",
            "p2tr", // TODO: set as a taproot address by default, make this configurable
        ];
        let output = run_command(options, "newaddress".to_owned(), commands)?;
        let found_address: Option<String> = lnd.get_property("address", output);
        if found_address.is_none() {
            error!(options.global_logger(), "no addess found");
            return Ok("".to_string());
        }
        Ok(found_address.unwrap())
    }

    pub fn open_channel(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        let _ = node.connect(options, node_command).map_err(|e| {
            debug!(options.global_logger(), "failed to connect: {}", e);
        });
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let amt = node_command.amt.unwrap_or(100000).to_string();
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();
        let to_pubkey = to_node.get_cached_pubkey();
        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "openchannel",
            &to_pubkey,
            &amt,
        ];
        let output = run_command(options, "openchannel".to_owned(), commands)?;
        if output.status.success() {
            info!(
                options.global_logger(),
                "successfully opened channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        } else {
            error!(
                options.global_logger(),
                "failed to open channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        }
        Ok(())
    }

    pub fn connect(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        info!(
            options.global_logger(),
            "to_node {} {} {} ",
            to_node.get_cached_pubkey(),
            to_node.get_p2p_port(),
            to_node.get_container_name()
        );
        let connection_url = to_node.get_connection_url();
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "connect",
            connection_url.as_ref(),
        ];
        let output = run_command(options, "connect".to_owned(), commands)?;

        if output.status.success()
            || from_utf8(&output.stderr)?.contains("already connected to peer")
        {
            info!(
                options.global_logger(),
                "successfully connected from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        } else {
            error!(
                options.global_logger(),
                "failed to connect from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        }
        Ok(())
    }

    pub fn close_channel(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        //TODO: add a user defined tag to channels to specify which channel to close, right now we just grab a random one for this peer
        let peer_channel_point = node.get_peers_channel_point(options, node_command)?;
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();
        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "closechannel",
            "--chan_point",
            peer_channel_point.as_ref(),
        ];
        let output = run_command(options, "closechannel".to_owned(), commands)?;

        if output.status.success() {
            info!(
                options.global_logger(),
                "successfully closed channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        } else {
            error!(
                options.global_logger(),
                "failed to close channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        }
        Ok(())
    }

    pub fn force_close_channel(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        //TODO: add a user defined tag to channels to specify which channel to close, right now we just grab a random one for this peer
        let peer_channel_point = node.get_peers_channel_point(options, node_command)?;
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();
        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "closechannel",
            "--force",
            "--chan_point",
            peer_channel_point.as_ref(),
        ];
        let output = run_command(options, "forceclosechannel".to_owned(), commands)?;

        if output.status.success() {
            info!(
                options.global_logger(),
                "successfully force closed channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        } else {
            error!(
                options.global_logger(),
                "failed to force close channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        }
        Ok(())
    }

    pub fn get_peers_channel_point(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();
        let to_pubkey = to_node.get_cached_pubkey();
        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "listchannels",
            "--peer",
            &to_pubkey,
        ];
        let output = run_command(options, "listchannels".to_owned(), commands)?;
        let channel_point = node.get_array_property("channels", "channel_point", output);
        if channel_point.is_none() {
            return Err(anyhow!("no channel point found!"));
        }
        Ok(channel_point.unwrap())
    }

    pub fn create_invoice(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        let amt = node_command.amt.unwrap_or(1000).to_string();
        let memo = node.generate_memo();
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "addinvoice",
            "--memo",
            memo.as_ref(),
            "--amt",
            amt.as_ref(),
        ];
        let output = run_command(options, "addinvoice".to_owned(), commands)?;
        let found_payment_request: Option<String> = node.get_property("payment_request", output);
        if found_payment_request.is_none() {
            error!(options.global_logger(), "no payment request found");
        }
        Ok(found_payment_request.unwrap())
    }

    pub fn pay_invoice(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
        payment_request: String,
    ) -> Result<(), Error> {
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let mut commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "payinvoice",
            "--pay_req",
            &payment_request,
            "-f",
        ];

        //This is an odd trick to get around lifetime issues
        let mut time_strings = Vec::new();
        if let Some(timeout) = node_command.timeout {
            commands.push("--timeout");
            let time_str = format!("{}s", timeout.clone());
            time_strings.push(time_str);
            let time_ref = time_strings.last().unwrap();
            commands.push(time_ref);
        }
        let output = run_command(options, "payinvoice".to_owned(), commands)?;
        if !output.status.success() {
            error!(
                options.global_logger(),
                "failed to make payment from {} to {}", node_command.from, node_command.to
            )
        }
        debug!(
            options.global_logger(),
            "output.stdout: {}, output.stderr: {}",
            from_utf8(&output.stdout)?,
            from_utf8(&output.stderr)?
        );
        Ok(())
    }

    pub fn pay_address(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
        address: &str,
    ) -> Result<String, Error> {
        let amt = node_command.amt.unwrap_or(1000).to_string();
        let subcommand = node_command.subcommand.to_owned().unwrap_or("".to_owned());
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "sendcoins",
            &subcommand,
            "--addr",
            address,
            "--amt",
            &amt,
        ];
        let output = run_command(options, "sendcoins".to_owned(), commands)?;
        let found_tx_id: Option<String> = node.get_property("txid", output);
        if found_tx_id.is_none() {
            error!(options.global_logger(), "no tx id found");
            return Ok("".to_owned());
        }

        Ok(found_tx_id.unwrap())
    }

    pub fn get_admin_macaroon(&self, node: &Lnd) -> Result<String, Error> {
        let macaroon_path: String = node.macaroon_path.clone();
        let mut file = OpenOptions::new().read(true).open(macaroon_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mac_as_hex = hex::encode(buffer);
        Ok(mac_as_hex)
    }

    pub fn get_rhash(&self, node: &Lnd, options: &Options) -> Result<String, Error> {
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "addinvoice",
        ];
        let output = run_command(options, "rhash".to_owned(), commands)?;
        let found_rhash: Option<String> = node.get_property("r_hash", output);
        if found_rhash.is_none() {
            error!(options.global_logger(), "no r_hash found");
            return Ok("".to_owned());
        }
        Ok(found_rhash.unwrap())
    }

    pub fn get_preimage(
        &self,
        node: &Lnd,
        options: &Options,
        rhash: String,
    ) -> Result<String, Error> {
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "lookupinvoice",
            &rhash,
        ];
        let output = run_command(options, "rpreimage".to_owned(), commands)?;
        let found_preimage: Option<String> = node.get_property("r_preimage", output);
        if found_preimage.is_none() {
            error!(options.global_logger(), "no preimage found");
            return Ok("".to_owned());
        }
        Ok(found_preimage.unwrap())
    }

    pub fn create_hold_invoice(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
        rhash: String,
    ) -> Result<String, Error> {
        let amt = node_command.amt.unwrap_or(1000).to_string();
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "addholdinvoice",
            &rhash,
            &amt,
        ];
        let output = run_command(options, "addholdinvoice".to_owned(), commands)?;
        let found_payment_request: Option<String> = node.get_property("payment_request", output);
        if found_payment_request.is_none() {
            error!(options.global_logger(), "no payment_request found");
            return Ok("".to_owned());
        }
        Ok(found_payment_request.unwrap())
    }

    pub fn settle_hold_invoice(
        &self,
        node: &Lnd,
        options: &Options,
        preimage: &String,
    ) -> Result<(), Error> {
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "settleinvoice",
            preimage,
        ];
        let output = run_command(options, "settleinvoice".to_owned(), commands)?;
        if output.status.success() {
            info!(options.global_logger(), "successfully settled invoice");
        } else {
            error!(options.global_logger(), "failed to settle invoice",);
        }
        Ok(())
    }

    pub fn get_current_block(&self, node: &Lnd, options: &Options) -> Result<i64, Error> {
        let rpc_command = node.get_rpc_server_command();
        let macaroon_path = node.get_macaroon_path();
        let compose_path = options.compose_path.as_ref().unwrap();

        let commands = vec![
            "-f",
            compose_path,
            "exec",
            "--user",
            "1000:1000",
            node.get_container_name(),
            "lncli",
            "--lnddir=/home/lnd/.lnd",
            "--network=regtest",
            macaroon_path,
            &rpc_command,
            "chain",
            "getbestblock",
        ];
        let output = run_command(options, "getbestblock".to_owned(), commands)?;
        if output.status.success() {
            info!(options.global_logger(), "successfully got getbestblock");
        } else {
            error!(options.global_logger(), "failed to got getbestblock");
        }
        let found_block_height: Option<i64> = node
            .get_property("block_height", output)
            .map(|s| s.parse::<i64>().unwrap());
        if found_block_height.is_none() {
            error!(options.global_logger(), "failed to get getbestblock");
            return Ok(0);
        }
        Ok(found_block_height.unwrap())
    }
}
