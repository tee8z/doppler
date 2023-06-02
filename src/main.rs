use anyhow::Result;
use doppler::{Commands, DopplerParser, NodeKind, Rule};
use log::info;
use pest::{iterators::Pair, Parser};
use std::fs;

fn main() {
    env_logger::init();

    let contents = fs::read_to_string("parsetest.doppler").expect("file read error");
    let parsed = DopplerParser::parse(Rule::page, &contents)
        .expect("parse error")
        .next()
        .unwrap();

    for line in parsed.into_inner() {
        match line.as_rule() {
            Rule::line => handle_line(line).expect("valid line"),
            Rule::EOI => return,
            _ => unreachable!(),
        }
    }
}

fn handle_line(line: Pair<Rule>) -> Result<()> {
    let mut inner = line.into_inner();
    let command = inner.next().expect("valid command");
    match command.try_into().expect("command pair") {
        Commands::Node => {
            let ident = inner.next().expect("ident").as_str();
            let kind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("valid node kind");
            handle_node_command(ident, kind)
        }
    }
}

fn handle_node_command(ident: &str, kind: NodeKind) -> Result<()> {
    info!("ident: {:?}, kind: {:?}", ident, kind);
    Ok(())
}
