use anyhow::bail;
use pest::iterators::Pair;
use std::convert::TryFrom;

use crate::Rule;

#[derive(Debug)]
pub enum NodeKind {
    Bitcoind,
    BitcoindMiner,
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
