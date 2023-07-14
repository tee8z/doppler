use anyhow::Result;
use doppler::{start_bitcoind, start_lnd, DopplerParser, EnvOption, NodeKind, Options, Rule};
use log::debug;
use pest::{iterators::Pair, Parser};
use std::fs;

fn main() {
    env_logger::init();

    let contents = fs::read_to_string("parsetest.doppler").expect("file read error");
    let parsed = DopplerParser::parse(Rule::page, &contents)
        .expect("parse error")
        .next()
        .unwrap();

    let mut options = Options::default();

    for line in parsed.into_inner() {
        match line.as_rule() {
            Rule::line => handle_line(&mut options, line).expect("valid line"),
            Rule::EOI => return,
            _ => unreachable!(),
        }
    }
}

fn handle_line(options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let mut line_inner = line.into_inner();
    debug!("line_inner: {:?}", line_inner);

    let command = line_inner.next().expect("valid command");
    let mut inner = command.clone().into_inner();
    match command.clone().as_rule() {
        Rule::node_command => {
            let ident = inner.next().expect("ident").as_str();
            let kind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("valid node kind");
            handle_node_command(options, ident, kind)
        }
        Rule::env_command => {
            let option = inner
                .next()
                .expect("option")
                .try_into()
                .expect("valid env option");
            let value = inner.next().expect("value").as_str();
            handle_env_command(options, option, value)
        }
        _ => Ok(()),
    }
}

fn handle_node_command(options: &mut Options, ident: &str, kind: NodeKind) -> Result<()> {
    debug!(
        "options: {:?}, ident: {:?}, kind: {:?}",
        options, ident, kind
    );
    match kind {
        NodeKind::Bitcoind => start_bitcoind(options, ident),
        NodeKind::LND => start_lnd(options, ident),
        _ => unimplemented!("deploying kind {:?} not implemented yet", kind),
    }
}

fn handle_env_command(options: &mut Options, option: EnvOption, value: &str) -> Result<()> {
    debug!(
        "options: {:?}, option: {:?}, value: {:?}",
        options, option, value
    );
    match option {
        EnvOption::DockerNetwork => {
            options.network_name = Some(value.to_owned());
            Ok(())
        }
    }
}
