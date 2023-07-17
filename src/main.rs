use anyhow::Result;
use doppler::{build_bitcoind, build_lnd, run_cluster, DopplerParser, NodeKind, Options, Rule};
use log::{debug, error};
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
            Rule::node_action => handle_action(&mut options, line).expect("invalid node action"),
            Rule::EOI => return,
            _ => unreachable!(),
        }
    }
}

fn handle_conf(options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let mut line_inner = line.into_inner();
    debug!("line_inner: {:?}", line_inner);
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
        Rule::pair_command => {
            let kind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            let ident = inner.next().expect("ident").as_str();
            let to_pair = inner.next().expect("invalid layer 1 node name").as_str();
            handle_build_command(options, ident, kind, Some(to_pair))?;
        }
        _ => (),
    }

    Ok(())
}

fn handle_build_command(
    options: &mut Options,
    ident: &str,
    kind: NodeKind,
    pair_name: Option<&str>,
) -> Result<()> {
    debug!(
        "options: {:?}, ident: {:?}, kind: {:?}, pair_name: {:?}",
        options, ident, kind, pair_name
    );
    match kind {
        NodeKind::Bitcoind => build_bitcoind(options, ident),
        NodeKind::LND => build_lnd(options, ident, pair_name.unwrap()),
        _ => unimplemented!("deploying kind {:?} not implemented yet", kind),
    }
}
fn handle_up(options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let line_inner = line.into_inner();
    debug!("line_inner: {:?}", line_inner);
    let default_compose = "./doppler-cluster.yaml";
    match run_cluster(options, default_compose) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to start cluster from generated compose file: {}", e);
            Ok(())
        }
    }
}

fn handle_action(_options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let mut line_inner = line.into_inner();
    debug!("line_inner: {:?}", line_inner);

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
