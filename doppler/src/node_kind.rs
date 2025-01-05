use anyhow::bail;
use pest::iterators::Pair;
use std::convert::TryFrom;

use crate::Rule;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Bitcoind,
    BitcoindMiner,
    #[default]
    Lnd,
    Coreln,
    Eclair,
}
impl<'a> TryFrom<Pair<'a, Rule>> for NodeKind {
    type Error = anyhow::Error;

    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::node_kind => match value.as_str() {
                "BITCOIND" => Ok(NodeKind::Bitcoind),
                "BITCOIND_MINER" => Ok(NodeKind::BitcoindMiner),
                "LND" => Ok(NodeKind::Lnd),
                "ECLAIR" => Ok(NodeKind::Eclair),
                "CORELN" => Ok(NodeKind::Coreln),
                _ => bail!("invalid node_kind"),
            },
            _ => bail!("pair should be a node_kind"),
        }
    }
}

impl From<LnNodeKind> for NodeKind {
    fn from(value: LnNodeKind) -> NodeKind {
        match value {
            LnNodeKind::Lnd => NodeKind::Lnd,
            LnNodeKind::Coreln => NodeKind::Coreln,
            LnNodeKind::Eclair => NodeKind::Eclair,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum BtcNodeKind {
    Bitcoind,
    #[default]
    BitcoindMiner,
}

impl<'a> TryFrom<Pair<'a, Rule>> for BtcNodeKind {
    type Error = anyhow::Error;

    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::node_kind => match value.as_str() {
                "BITCOIND" => Ok(BtcNodeKind::Bitcoind),
                "BITCOIND_MINER" => Ok(BtcNodeKind::BitcoindMiner),
                _ => bail!("invalid btc_node_kind"),
            },
            _ => bail!("pair should be a btc_node_kind"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum LnNodeKind {
    #[default]
    Lnd,
    Coreln,
    Eclair,
}

impl<'a> TryFrom<Pair<'a, Rule>> for LnNodeKind {
    type Error = anyhow::Error; // or whatever error type you're using

    fn try_from(pair: Pair<Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::ln_node_kind => match pair.as_str() {
                "LND" => Ok(LnNodeKind::Lnd),
                "ECLAIR" => Ok(LnNodeKind::Eclair),
                "CORELN" => Ok(LnNodeKind::Coreln),
                _ => Err(anyhow::anyhow!("Unknown ln node kind")),
            },
            _ => Err(anyhow::anyhow!(
                "invalid ln node kind: pair should be a ln_node_kind"
            )),
        }
    }
}
