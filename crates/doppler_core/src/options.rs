use anyhow::{anyhow, Error};
use std::sync::{
    atomic::{AtomicBool, AtomicI64},
    Arc, Mutex,
};

use crate::{L1Node, L2Node, Tag, Tags};

#[derive(Clone)]
pub struct Options {
    pub bitcoinds: Vec<Arc<dyn L1Node>>,
    pub lnd_nodes: Vec<Arc<dyn L2Node>>,
    pub eclair_nodes: Vec<Arc<dyn L2Node>>,
    pub cln_nodes: Vec<Arc<dyn L2Node>>,
    pub loop_count: Arc<AtomicI64>,
    pub tags: Arc<Mutex<Tags>>,
    pub rest: bool,
    pub external_nodes: Option<Vec<ExternalNode>>,
    pub network: String,
    pub read_end_of_doppler_script: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct ExternalNode {
    pub node_alias: String,
    pub macaroon_path: String,
    pub api_endpoint: String,
    pub tls_cert_path: String,
    pub network: String,
}

pub trait OptionLoad {
    fn load_bitcoinds(&mut self) -> Result<(), Error>;
    fn load_lnds(&mut self) -> Result<(), Error>;
    fn load_eclairs(&mut self) -> Result<(), Error>;
    fn load_coreln(&mut self) -> Result<(), Error>;
}

impl Options {
    pub fn get_l2_by_name(&self, name: &str) -> Result<Arc<dyn L2Node>, Error> {
        let lnd = self.lnd_nodes.iter().find(|node| node.get_name() == name);
        if lnd.is_none() {
            let eclair_node = self
                .eclair_nodes
                .iter()
                .find(|node| node.get_name() == name);
            if eclair_node.is_none() {
                let core_node = self.cln_nodes.iter().find(|node| node.get_name() == name);
                if core_node.is_none() {
                    return Err(anyhow!("node not found"));
                }
                return Ok(core_node.unwrap().to_owned());
            }
            return Ok(eclair_node.unwrap().to_owned());
        }
        Ok(lnd.unwrap().to_owned())
    }
    pub fn get_l2_nodes(&self) -> Vec<Arc<dyn L2Node>> {
        let mut l2_nodes: Vec<Arc<dyn L2Node>> = Vec::new();

        for lnd in self.lnd_nodes.iter() {
            l2_nodes.push(lnd.clone());
        }

        for eclair in self.eclair_nodes.iter() {
            l2_nodes.push(eclair.clone());
        }

        for coreln in self.cln_nodes.iter() {
            l2_nodes.push(coreln.clone());
        }

        l2_nodes
    }
    pub fn add_pubkeys_l2_nodes(&mut self) -> Result<(), Error> {
        let options_clone = self.clone();
        for lnd in self.lnd_nodes.iter() {
            lnd.add_pubkey(&options_clone);
        }

        for eclair in self.eclair_nodes.iter() {
            eclair.add_pubkey(&options_clone);
        }

        for coreln in self.cln_nodes.iter() {
            coreln.add_pubkey(&options_clone);
        }
        Ok(())
    }
    pub fn get_l1_by_name(&self, name: &str) -> Result<Arc<dyn L1Node>, Error> {
        let btcd = self
            .bitcoinds
            .iter()
            .find(|node| node.get_name() == *name)
            .unwrap_or_else(|| panic!("invalid node name: {:?}", name));
        Ok(btcd.clone())
    }
    pub fn save_tag(&self, tag: &Tag) -> Result<(), Error> {
        self.tags
            .lock()
            .unwrap()
            .save(tag.clone())
            .map_err(|e| e.into())
    }
    pub fn get_tag_by_name(&self, name: String) -> Tag {
        self.tags.lock().unwrap().get_by_name(name)
    }
}
