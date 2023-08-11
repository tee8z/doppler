use anyhow::Result;
use doppler::{build_bitcoind, build_lnd, run_cluster, DopplerParser, NodeKind, Options, Rule, MinerTime};
use log::{error, trace};
use pest::{iterators::Pair, Parser};
use std::fs;

fn main() {
    env_logger::init();
    //TODO: make path to doppler file able to be pased in via flag
    let contents = fs::read_to_string("parsetest.doppler").expect("file read error");
    let parsed = DopplerParser::parse(Rule::page, &contents)
        .expect("parse error")
        .next()
        .unwrap();

    let mut options = Options::new();

    for line in parsed.into_inner() {
        match line.as_rule() {
            Rule::conf => handle_conf(&mut options, line).expect("invalid conf line"),
            Rule::up => handle_up(&mut options, line).expect("invalid up line"),
            Rule::node_action_amt => handle_action_with_amt(&mut options, line).expect("invalid node action"),
            Rule::EOI => return,
            _ => unreachable!(),
        }
    }
}

fn handle_conf(options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let mut line_inner = line.into_inner();
    trace!("line_inner: {:?}", line_inner);
    let command = line_inner.next().expect("invalid command");
    let mut inner = command.clone().into_inner();

    match command.clone().as_rule() {
        Rule::node_def => {
            let kind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            let ident = inner.next().expect("ident").as_str();
            handle_build_command(options, ident, kind, None)?;
        }
        Rule::node_miner => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            let name = inner.next().expect("invalid ident").as_str();
            let time_num = inner
                .next()
                .expect("invalid time value")
                .as_str()
                .parse::<u64>()
                .expect("invalid time");
            let time_type = inner.next().expect("invalid time type").as_str().chars().nth(0).unwrap_or('\0');
            handle_build_command(options, name, kind, BuildDetails::new_miner_time(MinerTime::new(time_num, time_type)))?;
        }
        Rule::node_pair => {
            let kind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            let name = inner.next().expect("ident").as_str();
            let to_pair = inner.next().expect("invalid layer 1 node name").as_str();
            handle_build_command(options, name, kind, BuildDetails::new_pair(to_pair.to_owned()))?;
        }
        _ => (),
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct BuildDetails {
    pub pair_name: Option<String>,
    pub miner_time: Option<MinerTime>,
}

impl BuildDetails {
    pub fn new_pair(pair: String) -> Option<BuildDetails> {
        Some(BuildDetails{
            pair_name: Some(pair),
            miner_time: None
        })
    }
    pub fn new_miner_time(miner_time: MinerTime) -> Option<BuildDetails> {
        Some(BuildDetails {
            pair_name: None,
            miner_time: Some(miner_time)
        })
    }
}

fn handle_build_command(
    options: &mut Options,
    name: &str,
    kind: NodeKind,
    details: Option<BuildDetails>,
) -> Result<()> {
    trace!(
        "options: {:?}, name: {:?}, kind: {:?}, details: {:?}",
        options,
        name,
        kind,
        details
    );
    match kind {
        NodeKind::Bitcoind => build_bitcoind(options, name, None),
        NodeKind::BitcoindMiner => build_bitcoind(options, name, details.unwrap().miner_time),
        NodeKind::Lnd => build_lnd(options, name, details.unwrap().pair_name.unwrap().as_str()),
        _ => unimplemented!("deploying kind {:?} not implemented yet", kind),
    }
}
fn handle_up(options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let line_inner = line.into_inner();
    trace!("line_inner: {:?}", line_inner);
    let default_compose = "./doppler-cluster.yaml";
    match run_cluster(options, default_compose) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to start cluster from generated compose file: {}", e);
            Ok(())
        }
    }
}

fn handle_action_with_amt(_options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let mut line_inner = line.into_inner();
    trace!("line_inner: {:?}", line_inner);

    let _command = line_inner.next().expect("valid command");
    //let mut inner = command.clone().into_inner();
    /*  match command.clone().as_rule() {
        Rule::line => {
            let ident = inner.next().expect("ident").as_str();
            let kind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("valid node kind");
            handle_node_command(options, ident, kind)
        }
        _ => Ok(()),
    } */
    Ok(())
}
