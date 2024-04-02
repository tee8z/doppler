use crate::{generate_memo, L2Node, Lnd, NodeCommand, Options};
use anyhow::{anyhow, Error, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use hex::FromHex;
use reqwest::{
    blocking::{Client, Response},
    header::{HeaderMap, HeaderValue},
    Certificate, Method,
};
use serde_json::{json, Value};
use slog::{debug, error, info};
use std::{fs::OpenOptions, io::Read, thread, time::Duration};

#[derive(Debug, Clone)]
pub struct LndRest {
    base_url: String,
    macaroon_path: String,
    tls_path: String,
    pub client: Client,
}

impl Default for LndRest {
    fn default() -> Self {
        Self {
            base_url: String::from("http://localhost:8080"),
            macaroon_path: String::from(""),
            tls_path: String::from(""),
            client: Client::default(),
        }
    }
}

impl LndRest {
    pub fn new(base_url: &str, macaroon_path: String, tls_path: String) -> Result<Self, Error> {
        Ok(Self {
            base_url: base_url.to_owned(),
            macaroon_path: macaroon_path.to_owned(),
            tls_path: tls_path.to_owned(),
            client: Client::default(),
        })
    }
    fn build_url(&self, suburl: &str) -> String {
        format!("{}{}", self.base_url, suburl)
    }
    fn send_request(
        &self,
        options: &Options,
        command_name: String,
        method: Method,
        url: String,
        body: Option<String>,
        timeout: Option<u64>,
    ) -> Result<Response, Error> {
        //TODO: cleanup the nested if statements
        match method {
            Method::POST => {
                if let Some(body) = body {
                    info!(
                        options.global_logger(),
                        "({}): {} {}", command_name, url, body
                    );
                    if let Some(timeout) = timeout {
                        Ok(self
                            .client
                            .post(url)
                            .timeout(Duration::from_secs(timeout))
                            .body(body)
                            .send()?)
                    } else {
                        Ok(self.client.post(url).body(body).send()?)
                    }
                } else {
                    info!(options.global_logger(), "({}): {}", command_name, url);
                    Ok(self.client.post(url).send()?)
                }
            }
            Method::DELETE => {
                info!(options.global_logger(), "({}): {}", command_name, url);
                Ok(self.client.delete(url).send()?)
            }
            _ => {
                //Default to GET
                info!(options.global_logger(), "({}): {}", command_name, url);
                Ok(self.client.get(url).send()?)
            }
        }
    }
    pub fn get_node_pubkey(&self, options: &Options) -> Result<String, Error> {
        let url = self.build_url("/v1/getinfo");
        let mut retries = 3;
        let mut get_info_response = None;
        while retries > 0 {
            let response = self.send_request(
                options,
                "pubkey".to_owned(),
                Method::GET,
                url.clone(),
                None,
                None,
            )?;
            if !response.status().is_success() {
                debug!(options.global_logger(), "trying to get pubkey again");
                thread::sleep(Duration::from_secs(2));
                retries -= 1;
            } else {
                get_info_response = Some(response);
                break;
            }
        }

        if let Some(res) = get_info_response {
            let info: Value = res.json()?;
            if let Some(pubkey) = info.get("identity_pubkey").and_then(Value::as_str) {
                return Ok(pubkey.to_owned());
            } else {
                error!(options.global_logger(), "no pubkey found");
            }
        }

        Ok("".to_owned())
    }

    pub fn create_lnd_address(&self, options: &Options) -> Result<String, Error> {
        let url = self.build_url("/v1/newaddress?type=UNUSED_TAPROOT_PUBKEY");
        let result: Value = self
            .send_request(
                options,
                "newaddress".to_owned(),
                Method::GET,
                url,
                None,
                None,
            )?
            .json()?;
        if let Some(address) = result.get("address").and_then(Value::as_str) {
            return Ok(address.to_owned());
        }
        error!(options.global_logger(), "no address found: {}", result);
        return Ok("".to_string());
    }

    pub fn open_channel(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        let _ = node.connect(options, node_command).map_err(|e| {
            debug!(options.global_logger(), "failed to connect: {}", e);
        });
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let to_pubkey = to_node.get_cached_pubkey();
        let amt = node_command.amt.unwrap_or(100000).to_string();

        let url = self.build_url("/v1/channels");
        info!(options.global_logger(), "pubkey {}", to_pubkey);
        let to_pubkey_base64 = byte_base64_encoding(&to_pubkey)?;
        let body = json!({
            "node_pubkey":to_pubkey_base64,
            "local_funding_amount": amt
        });
        let mut retries = 3;
        let mut open_channel_response = None;
        while retries > 0 {
            let response = self.send_request(
                options,
                "openchannel".to_owned(),
                Method::POST,
                url.clone(),
                Some(body.to_string()),
                None,
            )?;
            if !response.status().is_success() {
                debug!(
                    options.global_logger(),
                    "trying to open channel again {}",
                    response.text()?
                );
                thread::sleep(Duration::from_secs(2));
                retries -= 1;
            } else {
                open_channel_response = Some(response);
                break;
            }
        }

        if let Some(result) = open_channel_response {
            if !result.status().is_success() {
                error!(
                    options.global_logger(),
                    "failed to open channel from {} to {}: {}",
                    node.get_name(),
                    to_node.get_name(),
                    result.text()?
                );
            } else {
                info!(
                    options.global_logger(),
                    "successfully opened channel from {} to {}",
                    node.get_name(),
                    to_node.get_name()
                )
            }
        } else {
            error!(
                options.global_logger(),
                "failed to open channel from {} to {}",
                node.get_name(),
                to_node.get_name(),
            );
        }

        Ok(())
    }

    pub fn connect(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        info!(
            options.global_logger(),
            "to_node {} {} {} ",
            to_node.get_cached_pubkey(),
            to_node.get_p2p_port(),
            to_node.get_container_name()
        );
        let url = self.build_url("/v1/peers");
        let body = json!({
            "addr": {
                "pubkey": &to_node.get_cached_pubkey(),
                "host": format!("{}:{}", to_node.get_container_name(),to_node.get_p2p_port())
            },
        });
        let result = self.send_request(
            options,
            "connect".to_owned(),
            Method::POST,
            url,
            Some(body.to_string()),
            None,
        )?;

        if result.status().is_success() {
            info!(
                options.global_logger(),
                "successfully connected from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
            return Ok(());
        }
        let result_text = result.text()?;
        if result_text.contains("already connected to peer") {
            info!(
                options.global_logger(),
                "successfully connected from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        } else {
            error!(
                options.global_logger(),
                "failed to connect from {} to {}: {}",
                node.get_name(),
                to_node.get_name(),
                result_text
            );
        }
        Ok(())
    }

    pub fn close_channel(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        //TODO: add a user defined tag to channels to specify which channel to close, right now we just grab a random one for this peer
        let peer_channel_point = node.get_peers_channel_point(options, node_command)?;
        let parts: Vec<&str> = peer_channel_point.split(':').collect();
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let sub_url = format!("/v1/channels/{}/{}", parts[0], parts[1]);
        let url = self.build_url(&sub_url);
        let result = self.send_request(
            options,
            "closechannel".to_owned(),
            Method::DELETE,
            url,
            None,
            None,
        )?;
        if result.status().is_success() {
            info!(
                options.global_logger(),
                "successfully closed channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        } else {
            error!(
                options.global_logger(),
                "failed to close channel from {} to {}: {}",
                node.get_name(),
                to_node.get_name(),
                result.text()?
            );
        }
        Ok(())
    }

    pub fn force_close_channel(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<(), Error> {
        //TODO: add a user defined tag to channels to specify which channel to close, right now we just grab a random one for this peer
        let peer_channel_point = node.get_peers_channel_point(options, node_command)?;
        let parts: Vec<&str> = peer_channel_point.split(':').collect();
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let sub_url = format!("/v1/channels/{}/{}?local_force=true", parts[0], parts[1]);
        let url = self.build_url(&sub_url);
        let result = self.send_request(
            options,
            "forceclosechannel".to_owned(),
            Method::DELETE,
            url,
            None,
            None,
        )?;
        if result.status().is_success() {
            info!(
                options.global_logger(),
                "successfully closed channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        } else {
            error!(
                options.global_logger(),
                "failed to close channel from {} to {}",
                node.get_name(),
                to_node.get_name()
            );
        }
        Ok(())
    }

    pub fn get_peers_channel_point(
        &self,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        let to_node = options.get_l2_by_name(node_command.to.as_str())?;
        let to_pubkey = to_node.get_cached_pubkey();
        let url = self.build_url("/v1/channels");
        let result: Value = self
            .send_request(
                options,
                "listchannels".to_owned(),
                Method::GET,
                url,
                None,
                None,
            )?
            .json()?;
        let channel_points = result
            .get("channels")
            .and_then(|channels| channels.as_array())
            .map(|channels| {
                channels
                    .iter()
                    .filter_map(|channel| {
                        let remote_key = channel
                            .get("remote_pubkey")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        if remote_key != to_pubkey {
                            return None;
                        }
                        channel
                            .get("channel_point")
                            .and_then(|point| point.as_str())
                            .map(|point| point.to_string())
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        if channel_points.is_empty() {
            return Err(anyhow!("no channel point found!: {}", result));
        }
        //TODO: should grab using a stored tag instead of first channel
        Ok(channel_points[0].clone())
    }

    pub fn create_invoice(
        &self,
        node: &Lnd,
        options: &Options,
        node_command: &NodeCommand,
    ) -> Result<String, Error> {
        let amt = node_command.amt.unwrap_or(1000).to_string();
        let memo = node.generate_memo();

        let body = json!({
            "memo":memo,
            "value": amt
        });
        let url = self.build_url("/v1/invoices");
        let result = self.send_request(
            options,
            "addinvoice".to_owned(),
            Method::POST,
            url,
            Some(body.to_string()),
            None,
        )?;
        if !result.status().is_success() {
            error!(options.global_logger(), "failed to create invoice");
            return Ok(String::from(""));
        }
        let response_payload: Value = result.json()?;
        let found_payment_request: Option<String> = response_payload
            .get("payment_request")
            .and_then(Value::as_str)
            .map(|payment| payment.to_owned());
        Ok(found_payment_request.unwrap())
    }

    pub fn pay_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        payment_request: String,
    ) -> Result<(), Error> {
        let url: String = self.build_url("/v1/channels/transactions");
        let body = json!({
                "payment_request":payment_request,
        });

        let result = self.send_request(
            options,
            "payinvoice".to_owned(),
            Method::POST,
            url,
            Some(body.to_string()),
            node_command.timeout,
        )?;
        if !result.status().is_success() {
            error!(
                options.global_logger(),
                "failed to make payment from {} to {}", node_command.from, node_command.to
            )
        }
        let result_text: Value = result.json()?;
        if let Some(error) = result_text.get("payment_error") {
            if error.is_string() && !error.as_str().unwrap().is_empty() {
                error!(
                    options.global_logger(),
                    "failed to make payment from {} to {}: {}",
                    node_command.from,
                    node_command.to,
                    result_text
                )
            }
        }
        debug!(
            options.global_logger(),
            "successful payment from {} to {}: {}", node_command.from, node_command.to, result_text
        );
        Ok(())
    }

    pub fn send_keysend(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        to_pubkey: String,
    ) -> Result<(), Error> {
        let url: String = self.build_url("/v1/channels/transactions");
        let memo = generate_memo();
        let r_hash = self.get_rhash(options)?;
        let preimage = self.get_preimage(options, r_hash.clone())?;

        let body = json!({
                "dest":base64_url_safe(&byte_base64_encoding(&to_pubkey)?),
                "amt":node_command.amt.unwrap_or(1000),
                "payment_hash":base64_url_safe(&byte_base64_encoding(&r_hash)?),
                "dest_custom_records": {
                    "5482373484": base64_url_safe(&byte_base64_encoding(&preimage)?),
                    "34349334": memo,
                }
        });

        let result = self.send_request(
            options,
            "send_keysend".to_owned(),
            Method::POST,
            url,
            Some(body.to_string()),
            node_command.timeout,
        )?;
        if !result.status().is_success() {
            error!(
                options.global_logger(),
                "failed to make payment from {} to {}: {}",
                node_command.from,
                node_command.to,
                result.text()?,
            );
            return Ok(());
        }
        let result_text: Value = result.json()?;
        if let Some(error) = result_text.get("payment_error") {
            if error.is_string() && !error.as_str().unwrap().is_empty() {
                error!(
                    options.global_logger(),
                    "failed to make payment from {} to {}: {}",
                    node_command.from,
                    node_command.to,
                    result_text
                )
            }
        }
        debug!(
            options.global_logger(),
            "successful payment from {} to {}: {}", node_command.from, node_command.to, result_text
        );
        Ok(())
    }

    pub fn pay_address(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        address: &str,
    ) -> Result<String, Error> {
        let amt = node_command.amt.unwrap_or(1000).to_string();
        let url: String = self.build_url("/v1/transactions");
        let body = json!({
                "addr":address,
                "amount": amt
        });
        let result = self.send_request(
            options,
            "sendcoins".to_owned(),
            Method::POST,
            url,
            Some(body.to_string()),
            None,
        )?;
        if !result.status().is_success() {
            error!(
                options.global_logger(),
                "failed to make chain-on payment from {} to {}", node_command.from, node_command.to
            )
        }
        let response_payload: Value = result.json()?;
        let found_tx_id: Option<String> = response_payload
            .get("txid")
            .and_then(Value::as_str)
            .map(|txid| txid.to_owned());
        if found_tx_id.is_none() {
            error!(
                options.global_logger(),
                "no tx id found: {}", response_payload
            );
            return Ok("".to_owned());
        }

        Ok(found_tx_id.unwrap())
    }

    pub fn get_admin_macaroon(&self, node: &Lnd) -> Result<String, Error> {
        let mac_as_hex = get_admin_macaroon(node.macaroon_path.clone())?;
        Ok(mac_as_hex)
    }

    pub fn get_rhash(&self, options: &Options) -> Result<String, Error> {
        let url: String = self.build_url("/v1/invoices");
        let result =
            self.send_request(options, "rhash".to_owned(), Method::POST, url, None, None)?;
        if !result.status().is_success() {
            error!(
                options.global_logger(),
                "failed to get rhash from empty invoice"
            )
        }
        let response_payload: Value = result.json()?;
        let found_rhash_base64: Option<String> = response_payload
            .get("r_hash")
            .and_then(Value::as_str)
            .map(|rhash| rhash.to_owned());

        if found_rhash_base64.is_none() {
            error!(options.global_logger(), "no r_hash found");
            return Ok("".to_owned());
        }
        let rhash_hex = hex_encoding(&found_rhash_base64.unwrap())?;
        Ok(rhash_hex)
    }

    pub fn get_preimage(&self, options: &Options, rhash: String) -> Result<String, Error> {
        let sub_url = format!("/v1/invoice/{}", rhash);
        let url = self.build_url(&sub_url);
        let result = self.send_request(
            options,
            "rpreimage".to_owned(),
            Method::GET,
            url,
            None,
            None,
        )?;
        if !result.status().is_success() {
            error!(
                options.global_logger(),
                "failed to get preimage of invoice: {}",
                result.text()?
            );
            return Ok("".to_owned());
        }
        let response_payload: Value = result.json()?;
        let found_preimage_base64: Option<String> = response_payload
            .get("r_preimage")
            .and_then(Value::as_str)
            .map(|preimage| preimage.to_owned());
        if found_preimage_base64.is_none() {
            error!(options.global_logger(), "no preimage found");
            return Ok("".to_owned());
        }
        let preimage_hex = hex_encoding(&found_preimage_base64.unwrap())?;
        Ok(preimage_hex)
    }

    pub fn create_hold_invoice(
        &self,
        options: &Options,
        node_command: &NodeCommand,
        rhash: String,
    ) -> Result<String, Error> {
        let amt = node_command.amt.unwrap_or(1000).to_string();
        let url = self.build_url("/v2/invoices/hodl");
        let rhash_base64 = byte_base64_encoding(&rhash)?;
        let body = json!({
            "value": amt,
            "hash": base64_url_safe(&rhash_base64)
        });
        let result = self.send_request(
            options,
            "addholdinvoice".to_owned(),
            Method::POST,
            url,
            Some(body.to_string()),
            None,
        )?;
        if !result.status().is_success() {
            error!(options.global_logger(), "failed to create invoice");
            return Ok(String::from(""));
        }
        let response_payload: Value = result.json()?;
        let found_payment_request: Option<String> = response_payload
            .get("payment_request")
            .and_then(Value::as_str)
            .map(|payment: &str| payment.to_owned());
        Ok(found_payment_request.unwrap())
    }

    pub fn settle_hold_invoice(&self, options: &Options, preimage: &String) -> Result<(), Error> {
        let url = self.build_url("/v2/invoices/settle");
        let preimage_base64 = byte_base64_encoding(&preimage)?;
        let body = json!({
            "preimage": base64_url_safe(&preimage_base64)
        });
        let result = self.send_request(
            options,
            "settleinvoice".to_owned(),
            Method::POST,
            url,
            Some(body.to_string()),
            None,
        )?;
        if result.status().is_success() {
            info!(options.global_logger(), "successfully settled invoice");
        } else {
            error!(options.global_logger(), "failed to settle invoice",);
        }
        Ok(())
    }
    pub fn get_current_block(&self, options: &Options) -> Result<i64, Error> {
        let url = self.build_url("/v2/chainkit/bestblock");
        let result = self.send_request(
            options,
            "getbestblock".to_owned(),
            Method::GET,
            url,
            None,
            None,
        )?;
        if !result.status().is_success() {
            error!(
                options.global_logger(),
                "failed to get getbestblock: {}",
                result.text()?
            );
            return Ok(0);
        }
        let response_payload: Value = result.json()?;
        let found_block_height: Option<i64> =
            response_payload.get("block_height").and_then(Value::as_i64);
        Ok(found_block_height.unwrap())
    }
}

fn get_admin_macaroon(macaroon_path: String) -> Result<String, Error> {
    let mut file = OpenOptions::new().read(true).open(macaroon_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let mac_as_hex = hex::encode(buffer);
    Ok(mac_as_hex)
}

fn get_tls_cert(tls_path: String) -> Result<Certificate, Error> {
    let mut file = OpenOptions::new().read(true).open(tls_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Certificate::from_pem(&buffer).map_err(|e| e.into())
}

pub fn add_rest_client(rest_client: LndRest) -> Result<LndRest, Error> {
    let mut rest_client_cpy = rest_client.clone();
    let macaroon_hex = get_admin_macaroon(rest_client.macaroon_path)?;
    let mut auth_value = HeaderValue::from_str(&macaroon_hex)?;
    auth_value.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert("Grpc-Metadata-macaroon", auth_value);
    let mut client_builder = reqwest::blocking::Client::builder().default_headers(headers);
    if !rest_client.tls_path.is_empty() {
        let cert = get_tls_cert(rest_client.tls_path)?;
        client_builder = client_builder.add_root_certificate(cert);
    }
    rest_client_cpy.client = client_builder.build()?;
    Ok(rest_client_cpy)
}

pub fn base64_url_safe(base64_str: &str) -> String {
    base64_str.replace("+", "-").replace("/", "_")
}

pub fn byte_base64_encoding(hex_str: &str) -> Result<String, Error> {
    let pubkey_bytes = Vec::from_hex(hex_str)?;
    Ok(BASE64_STANDARD.encode(&pubkey_bytes))
}

pub fn hex_encoding(byte_base64: &str) -> Result<String, Error> {
    let bytes = BASE64_STANDARD.decode(byte_base64)?;
    Ok(hex::encode(bytes))
}
