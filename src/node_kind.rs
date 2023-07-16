use crate::Rule;
use anyhow::bail;
use pest::iterators::Pair;
use std::convert::TryFrom;

#[derive(Debug)]
pub enum NodeKind {
    Bitcoind,
    LND,
    CORELN,
    ECLAIR,
}

impl<'a> TryFrom<Pair<'a, Rule>> for NodeKind {
    type Error = anyhow::Error;

    fn try_from(value: Pair<Rule>) -> Result<Self, Self::Error> {
        match value.as_rule() {
            Rule::node_kind => match value.as_str() {
                "BITCOIND" => Ok(NodeKind::Bitcoind),
                "LND" => Ok(NodeKind::LND),
                "ECLAIR" => Ok(NodeKind::ECLAIR),
                "CORELN" => Ok(NodeKind::CORELN),
                _ => bail!("invalid node_kind"),
            },
            _ => bail!("pair should be a node_kind"),
        }
    }
}
