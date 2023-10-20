use anyhow::{anyhow, Error};
use reqwest::blocking::get;
use serde::Deserialize;

use crate::{CloneableHashMap, ImageInfo, NodeKind};

#[derive(Debug, Deserialize)]
struct Image {
    latest: String,
    #[serde(rename = "versions")]
    _versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Images {
    #[serde(rename = "LND")]
    lnd: Image,
    #[serde(rename = "c-lightning")]
    c_lightning: Image,
    eclair: Image,
    bitcoind: Image,
}

#[derive(Debug, Deserialize)]
struct Payload {
    #[serde(rename = "version")]
    _version: i32,
    images: Images,
}

pub fn get_latest_polar_images() -> Result<CloneableHashMap<NodeKind, ImageInfo>, Error> {
    let url = "https://raw.githubusercontent.com/jamaljsr/polar/master/docker/nodes.json";

    let response = get(url).map_err(|err| anyhow!("error getting polar images: {}", err))?;
    if response.status().is_success() {
        let payload: Payload = response.json::<Payload>().expect("Failed to parse JSON");
        let mut hash_map = CloneableHashMap::new();
        // NOTE: safe to use * as name since the grammar of the parse wont allow for special characters for the image name, only for the image tag
        hash_map.insert(
            NodeKind::Lnd,
            ImageInfo::new(
                payload.images.lnd.latest,
                String::from("*1"),
                false,
                NodeKind::Lnd,
            ),
        );
        hash_map.insert(
            NodeKind::Coreln,
            ImageInfo::new(
                payload.images.c_lightning.latest,
                String::from("*2"),
                false,
                NodeKind::Coreln,
            ),
        );
        hash_map.insert(
            NodeKind::Eclair,
            ImageInfo::new(
                payload.images.eclair.latest,
                String::from("*3"),
                false,
                NodeKind::Eclair,
            ),
        );
        hash_map.insert(
            NodeKind::Bitcoind,
            ImageInfo::new(
                payload.images.bitcoind.latest.clone(),
                String::from("*4"),
                false,
                NodeKind::Bitcoind,
            ),
        );
        hash_map.insert(
            NodeKind::BitcoindMiner,
            ImageInfo::new(
                payload.images.bitcoind.latest,
                String::from("*5"),
                false,
                NodeKind::BitcoindMiner,
            ),
        );
        Ok(hash_map)
    } else {
        Err(anyhow!(
            "HTTP request failed with status: {}",
            response.status()
        ))
    }
}

pub fn get_polar_images() -> Result<CloneableHashMap<NodeKind, Vec<ImageInfo>>, Error> {
    let url = "https://raw.githubusercontent.com/jamaljsr/polar/master/docker/nodes.json";

    let response = get(url).map_err(|err| anyhow!("error getting polar images: {}", err))?;
    if response.status().is_success() {
        let payload: Payload = response.json::<Payload>().expect("Failed to parse JSON");
        let mut hash_map: CloneableHashMap<NodeKind, Vec<ImageInfo>> = CloneableHashMap::new();
        // NOTE: safe to use * as name since the grammar of the parse wont allow for special characters for the image name, only for the image tag

        let lnd_versions: Vec<ImageInfo> = payload
            .images
            .lnd
            ._versions
            .iter()
            .enumerate()
            .map(|(index, version)| {
                ImageInfo::new(version.to_owned(), format!("*_lnd-{}", index), false, NodeKind::Lnd)
            })
            .collect();
        hash_map.insert(NodeKind::Lnd, lnd_versions);

        let c_lightning_versions: Vec<ImageInfo> = payload
            .images
            .c_lightning
            ._versions
            .iter()
            .enumerate()
            .map(|(index, version)| {
                ImageInfo::new(version.to_owned(), format!("*_clightning-{}", index), false, NodeKind::Coreln)
            })
            .collect();
        hash_map.insert(NodeKind::Coreln, c_lightning_versions);

        let eclair_versions: Vec<ImageInfo> = payload
            .images
            .eclair
            ._versions
            .iter()
            .enumerate()
            .map(|(index, version)| {
                ImageInfo::new(version.to_owned(), format!("*_eclair-{}", index), false, NodeKind::Eclair)
            })
            .collect();

        hash_map.insert(NodeKind::Eclair, eclair_versions);
        let bitcoind_versions: Vec<ImageInfo> = payload
            .images
            .bitcoind
            ._versions
            .iter()
            .enumerate()
            .map(|(index, version)| {
                ImageInfo::new(version.to_owned(), format!("*_bitcoind-{}", index), false, NodeKind::Bitcoind)
            })
            .collect();

        hash_map.insert(NodeKind::Bitcoind, bitcoind_versions.clone());
        hash_map.insert(NodeKind::BitcoindMiner, bitcoind_versions);
        Ok(hash_map)
    } else {
        Err(anyhow!(
            "HTTP request failed with status: {}",
            response.status()
        ))
    }
}
