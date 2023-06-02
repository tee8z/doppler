use crate::Rule;
use anyhow::bail;
use pest::iterators::Pair;
use std::convert::TryFrom;

#[derive(Debug)]
pub enum Commands {
    Node,
}

impl<'a> TryFrom<Pair<'a, Rule>> for Commands {
    type Error = anyhow::Error;

    // Required method
    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::command => match value.as_str() {
                "NODE" => Ok(Commands::Node),
                _ => bail!("invalid command"),
            },
            _ => bail!("pair should be a command"),
        }
    }
}

#[derive(Debug)]
pub enum NodeKind {
    Bitcoind,
    LND,
}

impl<'a> TryFrom<Pair<'a, Rule>> for NodeKind {
    type Error = anyhow::Error;

    // Required method
    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::node_kind => match value.as_str() {
                "BITCOIND" => Ok(NodeKind::Bitcoind),
                "LND" => Ok(NodeKind::LND),
                _ => bail!("invalid command"),
            },
            _ => bail!("pair should be a command"),
        }
    }
}
