use crate::Rule;
use anyhow::bail;
use pest::iterators::Pair;
use std::convert::TryFrom;

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
                _ => bail!("invalid node_kind"),
            },
            _ => bail!("pair should be a node_kind"),
        }
    }
}

#[derive(Debug)]
pub enum EnvOption {
    DockerNetwork,
}

impl<'a> TryFrom<Pair<'a, Rule>> for EnvOption {
    type Error = anyhow::Error;

    // Required method
    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::env_option => match value.as_str() {
                "DOCKER_NETWORK" => Ok(EnvOption::DockerNetwork),
                _ => bail!("invalid env_option"),
            },
            _ => bail!("pair should be a env_option"),
        }
    }
}
