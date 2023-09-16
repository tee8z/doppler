use crate::{Bitcoind, Options};
use anyhow::Error;
use rand::Rng;
use serde_yaml::{from_slice, Value};
use slog::info;
use std::{any::Any, process::Output};

pub trait L2Node: Any {
    fn get_connection_url(&self) -> String;
    fn get_p2p_port(&self) -> &str;
    fn get_name(&self) -> &str;
    fn get_alias(&self) -> &str;
    fn get_server_url(&self) -> &str;
    fn get_container_name(&self) -> &str;
    fn get_cached_pubkey(&self) -> String;
    fn add_pubkey(&mut self, option: &Options);
    fn get_node_pubkey(&self, options: &Options) -> Result<String, Error>;
    fn open_channel(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error>;
    fn connect(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error>;
    fn close_channel(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error>;
    fn create_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error>;
    fn pay_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        payment_request: String,
    ) -> Result<(), Error>;
    fn create_on_chain_address(&self, options: &Options) -> Result<String, Error>;
    fn pay_address(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        address: &str,
    ) -> Result<String, Error>;
    fn send_ln(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        let to_node = options.get_l2_by_name(&node_command.to)?;
        let invoice = to_node.create_invoice(options, node_command)?;
        self.pay_invoice(options, node_command, invoice)?;
        Ok(())
    }
    fn send_on_chain(&self, options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
        let to_node = options.get_l2_by_name(&node_command.to)?;
        let on_chain_address_from = to_node.create_on_chain_address(options)?;
        let tx_id = self.pay_address(options, node_command, on_chain_address_from.as_str())?;
        info!(
            options.global_logger(),
            "on chain transaction created: {}", tx_id
        );
        Ok(())
    }
    fn fund_node(&self, options: &Options, miner: &Bitcoind) -> Result<(), Error> {
        let address = self.create_on_chain_address(options)?;
        miner.clone().mine_to_address(options, 2, address)?;
        Ok(())
    }
    fn get_property(&self, name: &str, output: Output) -> Option<String> {
        get_property(name, output)
    }
    fn get_array_property(
        &self,
        array_name: &str,
        inner_property: &str,
        output: Output,
    ) -> Option<String> {
        get_array_property(array_name, inner_property, output)
    }
    fn generate_memo(&self) -> String {
        generate_memo()
    }
}

pub trait L1Node: Any {
    fn start_mining(&self, options: &Options) -> Result<(), Error>;
    fn mine_bitcoin_continously(&self, options: &Options);
    fn mine_bitcoin(&self, options: &Options, num_blocks: i64) -> Result<String, Error>;
    fn create_wallet(&self, options: &Options) -> Result<(), Error>;
    fn load_wallet(&self, options: &Options) -> Result<(), Error>;
    fn get_name(&self) -> String;
    fn get_container_name(&self) -> String;
    fn get_data_dir(&self) -> String;
    fn get_miner_time(&self) -> &Option<MinerTime>;
    fn get_zmqpubrawblock(&self) -> String;
    fn get_zmqpubhashblock(&self) -> String;
    fn get_zmqpubrawtx(&self) -> String;
    fn get_rpc_username(&self) -> String;
    fn get_rpc_password(&self) -> String;
    fn get_rpc_port(&self) -> String;
    fn get_p2p_port(&self) -> String;
    fn create_address(&self, options: &Options) -> Result<String, Error>;
    fn mine_to_address(
        self,
        options: &Options,
        num_blocks: i64,
        address: String,
    ) -> Result<(), Error>;
}

#[derive(Default, Debug, Clone)]
pub struct NodeCommand {
    pub name: String,
    pub from: String,
    pub to: String,
    pub amt: Option<i64>,
    pub subcommand: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct MinerTime {
    pub miner_interval_amt: u64,
    pub miner_interval_type: char,
}

impl MinerTime {
    pub fn new(amt: u64, time_type: char) -> MinerTime {
        MinerTime {
            miner_interval_amt: amt,
            miner_interval_type: time_type,
        }
    }
}

pub fn generate_memo() -> String {
    let words = [
        "piano",
        "balance",
        "transaction",
        "exchange",
        "receipt",
        "wire",
        "deposit",
        "wallet",
        "sats",
        "profit",
        "transfer",
        "vendor",
        "investment",
        "payment",
        "debit",
        "card",
        "bank",
        "account",
        "money",
        "order",
        "gateway",
        "online",
        "confirmation",
        "interest",
        "fraud",
        "Olivia",
        "Elijah",
        "Ava",
        "Liam",
        "Isabella",
        "Mason",
        "Sophia",
        "William",
        "Emma",
        "James",
        "parrot",
        "dolphin",
        "breeze",
        "moonlight",
        "whisper",
        "velvet",
        "marble",
        "sunset",
        "seashell",
        "peacock",
        "rainbow",
        "guitar",
        "harmony",
        "lulla",
        "crystal",
        "butterfly",
        "stardust",
        "cascade",
        "serenade",
        "lighthouse",
        "orchid",
        "sapphire",
        "silhouette",
        "tulip",
        "firefly",
        "brook",
        "feather",
        "mermaid",
        "twilight",
        "dandelion",
        "morning",
        "serenity",
        "emerald",
        "flamingo",
        "gazelle",
        "ocean",
        "carousel",
        "sparkle",
        "dewdrop",
        "paradise",
        "polaris",
        "meadow",
        "quartz",
        "zenith",
        "horizon",
        "sunflower",
        "melody",
        "trinket",
        "whisker",
        "cabana",
        "harp",
        "blossom",
        "jubilee",
        "raindrop",
        "sunrise",
        "zeppelin",
        "whistle",
        "ebony",
        "gardenia",
        "lily",
        "marigold",
        "panther",
        "starlight",
        "harmonica",
        "shimmer",
        "canary",
        "comet",
        "moonstone",
        "rainforest",
        "buttercup",
        "zephyr",
        "violet",
        "serenade",
        "swan",
        "pebble",
        "coral",
        "radiance",
        "violin",
        "zodiac",
        "serenade",
    ];

    let mut rng = rand::thread_rng();
    let random_index = rng.gen_range(0..words.len());
    let mut memo = String::new();
    let limit = rng.gen_range(1..=15);
    for (index, word) in words.iter().enumerate() {
        if index >= limit {
            break;
        }
        if !memo.is_empty() {
            memo.push(' ');
        }
        memo.push_str(word);
    }
    let random_word = words[random_index];
    random_word.to_owned()
}

fn get_property(name: &str, output: Output) -> Option<String> {
    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        match response
            .as_mapping()
            .and_then(|obj| obj.get(name))
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            Some(value) => return Some(value),
            None => return None,
        }
    }
    None
}

fn get_array_property(array_name: &str, inner_property: &str, output: Output) -> Option<String> {
    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        if let Some(value) = response
            .as_mapping()
            .and_then(|obj| obj.get(array_name))
            .and_then(|array| array.as_sequence())
            .and_then(|array| array.first())
            .and_then(|obj| obj.get(inner_property))
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            return Some(value);
        } else {
            return None;
        }
    }
    None
}
