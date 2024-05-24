use anyhow::{Error, Result};
use doppler_core::{ImageInfo, MinerTime, NodeKind, Options};
use doppler_parser::Rule;
use log::error;
use pest::iterators::Pair;

use crate::{build_bitcoind, build_cln, build_eclair, build_lnd, run_cluster, Daemon};

const COMPOSE_PATH: &str = "doppler-cluster.yaml";

#[derive(Debug, Default)]
pub struct BuildDetails {
    pub pair: Option<NodePair>,
    pub miner_time: Option<MinerTime>,
}

#[derive(Debug, Default)]
pub struct NodePair {
    pub name: String,
    pub wallet_starting_balance: i64,
}

impl BuildDetails {
    pub fn new_pair(pair: String, amount: i64) -> Option<BuildDetails> {
        Some(BuildDetails {
            pair: Some(NodePair {
                name: pair,
                wallet_starting_balance: amount,
            }),
            miner_time: None,
        })
    }
    pub fn new_miner_time(miner_time: MinerTime) -> Option<BuildDetails> {
        Some(BuildDetails {
            pair: None,
            miner_time: Some(miner_time),
        })
    }
}


pub fn handle_build_command(
    options: &mut Options,
    name: &str,
    kind: NodeKind,
    image: &ImageInfo,
    details: Option<BuildDetails>,
) -> Result<()> {

    match kind {
        NodeKind::Bitcoind => build_bitcoind(options, name, image, false),
        NodeKind::BitcoindMiner => build_bitcoind(options, name, image, true),
        NodeKind::Lnd => build_lnd(options, name, image, &details.unwrap().pair.unwrap()),
        NodeKind::Eclair => build_eclair(options, name, image, &details.unwrap().pair.unwrap()),
        NodeKind::Coreln => build_cln(options, name, image, &details.unwrap().pair.unwrap()),
    }
}

pub fn handle_up(options: &mut Options) -> Result<(), Error> {
    run_cluster(options, COMPOSE_PATH).map_err(|e| {
        error!("Failed to start cluster from generated compose file: {}", e);
        e
    })?;
    Ok(())
}


pub fn handle_image_command(
    option: &mut Daemon,
    kind: NodeKind,
    name: &str,
    tag_or_path: &str,
) -> Result<()> {
    let is_known_image = option.is_known_polar_image(kind.clone(), name, tag_or_path);

    let image = ImageInfo::new(
        tag_or_path.to_owned(),
        name.to_owned(),
        !is_known_image,
        kind,
    );
    option.images.push(image);
    Ok(())
}


pub fn get_image(options: &mut Daemon, node_kind: NodeKind, possible_name: &str) -> ImageInfo {
    let image_info = if !possible_name.is_empty() {
        if let Some(image) = options.get_image(possible_name) {
            image
        } else {
            options.get_default_image(node_kind)
        }
    } else {
        options.get_default_image(node_kind)
    };
    image_info
}


pub fn handle_conf(options: &mut Daemon, line: Pair<Rule>) -> Result<()> {
    let mut line_inner = line.into_inner();
    let command = line_inner.next().expect("invalid command");
    let mut inner = command.clone().into_inner();

    match command.clone().as_rule() {
        Rule::node_image => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            if options.external_nodes.is_some() && kind != NodeKind::Lnd {
                unimplemented!("can only support LND nodes at the moment for remote nodes");
            }
            let image_name = inner.next().expect("image name").as_str();
            let tag_or_path = inner.next().expect("image version").as_str();
            handle_image_command(options, kind, image_name, tag_or_path)?;
        }
        Rule::node_def => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            if options.external_nodes.is_some() && kind != NodeKind::Lnd {
                unimplemented!("can only support LND nodes at the moment for remote nodes");
            }
            let node_name = inner.next().expect("node name").as_str();
            let image: ImageInfo = match inner.next() {
                Some(image) => get_image(options, kind.clone(), image.as_str()),
                None => options.get_default_image(kind.clone()),
            };
            handle_build_command(options, node_name, kind, &image, None)?;
        }
        Rule::node_pair => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            if options.external_nodes.is_some() && kind != NodeKind::Lnd {
                unimplemented!("can only support LND nodes at the moment for remote nodes");
            }
            let name = inner.next().expect("ident").as_str();
            let image = match inner.peek().unwrap().as_rule() {
                Rule::image_name => {
                    let image_name = inner.next().expect("image name").as_str();
                    get_image(options, kind.clone(), image_name)
                }
                _ => options.get_default_image(kind.clone()),
            };
            let to_pair = inner.next().expect("invalid layer 1 node name").as_str();
            let amount = match inner.peek().is_some() {
                true => inner
                    .next()
                    .expect("invalid amount")
                    .as_str()
                    .parse()
                    .unwrap(),
                false => 100000000,
            };
            handle_build_command(
                options,
                name,
                kind,
                &image,
                BuildDetails::new_pair(to_pair.to_owned(), amount),
            )?;
        }
        _ => (),
    }

    Ok(())
}
