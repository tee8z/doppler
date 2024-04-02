use crate::{run_command, Bitcoind, NodeKind, Options};
use anyhow::Error;
use rand::Rng;
use serde_yaml::{from_slice, Value};
use slog::info;
use std::{any::Any, process::Output};

pub trait L2Node: Any {
    fn stop(&self, options: &Options) -> Result<(), Error> {
        let container_name = self.get_container_name();
        let compose_path = options.compose_path.as_ref().unwrap();
        let commands = vec!["-f", &compose_path, "stop", &container_name];
        run_command(options, String::from("stop"), commands).map(|_| ())
    }
    fn start(&self, options: &Options) -> Result<(), Error> {
        let container_name = self.get_container_name();
        let compose_path = options.compose_path.as_ref().unwrap();
        let commands = vec!["-f", &compose_path, "start", &container_name];
        run_command(options, String::from("start"), commands).map(|_| ())
    }
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
    fn get_rhash(&self, option: &Options) -> Result<String, Error>;
    fn get_preimage(&self, option: &Options, rhash: String) -> Result<String, Error>;
    fn force_close_channel(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error>;
    fn get_starting_wallet_balance(&self) -> i64;
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
        if let Some(command) = node_command.subcommand.clone() {
            if command == "--keysend" {
                return self.send_keysend(options, node_command, to_node.get_cached_pubkey());
            }
        }
        let invoice = to_node.create_invoice(options, node_command)?;
        self.pay_invoice(options, node_command, invoice)?;
        Ok(())
    }
    fn send_keysend(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        to_pubkey: String,
    ) -> Result<(), Error>;
    fn create_hold_invoice(
        &self,
        option: &Options,
        node_command: &NodeCommand,
        rhash: String,
    ) -> Result<String, Error>;
    fn settle_hold_invoice(&self, options: &Options, preimage: String) -> Result<(), Error>;
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
        miner
            .clone()
            .send_to_address(options, 1, self.get_starting_wallet_balance(), address)
    }
    fn get_property(&self, name: &str, output: Output) -> Option<String> {
        get_property(name, output)
    }
    fn get_property_num(&self, name: &str, output: Output) -> Option<i64> {
        get_property_num(name, output)
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
    fn wait_for_block(&self, options: &Options, num_of_blocks: i64) -> Result<(), Error>;
}

pub trait L1Node: Any {
    fn stop(&self, options: &Options) -> Result<(), Error> {
        let container_name = self.get_container_name();
        let compose_path = options.compose_path.as_ref().unwrap();
        let commands = vec!["-f", &compose_path, "stop", &container_name];
        run_command(options, String::from("stop"), commands).map(|_| ())
    }
    fn start(&self, options: &Options) -> Result<(), Error> {
        let container_name = self.get_container_name();
        let compose_path = options.compose_path.as_ref().unwrap();
        let commands = vec!["-f", &compose_path, "start", &container_name];
        run_command(options, String::from("start"), commands).map(|_| ())
    }
    fn mine_bitcoin(&self, options: &Options, num_blocks: i64) -> Result<String, Error>;
    fn create_wallet(&self, options: &Options) -> Result<(), Error>;
    fn load_wallet(&self, options: &Options) -> Result<(), Error>;
    fn get_name(&self) -> String;
    fn get_container_name(&self) -> String;
    fn get_data_dir(&self) -> String;
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
    fn send_to_address(
        self,
        options: &Options,
        num_blocks: i64,
        amt: i64,
        address: String,
    ) -> Result<(), Error>;
    fn send_to_l2(self, options: &Options, node_command: &NodeCommand) -> Result<(), Error>;
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ImageInfo {
    tag: String,
    name: String,
    is_custom: bool,
    node_kind: NodeKind,
}

impl ImageInfo {
    pub fn new(tag: String, name: String, is_custom: bool, node_kind: NodeKind) -> ImageInfo {
        ImageInfo {
            tag,
            name,
            is_custom,
            node_kind,
        }
    }
    pub fn get_image(&self) -> String {
        if self.is_custom {
            self.tag.clone()
        } else {
            match self.node_kind {
                NodeKind::Lnd => {
                    format!("polarlightning/lnd:{}", self.tag.clone())
                }
                NodeKind::Bitcoind | NodeKind::BitcoindMiner => {
                    format!("polarlightning/bitcoind:{}", self.tag.clone())
                }
                NodeKind::Coreln => {
                    format!("polarlightning/clightning:{}", self.tag.clone())
                }
                NodeKind::Eclair => {
                    format!("polarlightning/eclair:{}", self.tag.clone())
                }
                NodeKind::Visualizer => self.tag.clone(),
            }
        }
    }
    pub fn is_image(&self, name: &str) -> bool {
        self.name == name
    }
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    pub fn get_tag(&self) -> String {
        self.tag.clone()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NodeCommand {
    pub name: String,
    pub from: String,
    pub to: String,
    pub amt: Option<i64>,
    pub subcommand: Option<String>,
    // used to coordinate channel or hold invoice
    pub tag: Option<String>,
    // timeout in seconds
    pub timeout: Option<u64>,
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

fn get_property_num(name: &str, output: Output) -> Option<i64> {
    if output.status.success() {
        let response: Value = from_slice(&output.stdout).expect("failed to parse JSON");
        match response
            .as_mapping()
            .and_then(|obj| obj.get(name))
            .and_then(Value::as_i64)
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
