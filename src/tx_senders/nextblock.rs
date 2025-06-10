const MODULE: &str = "NexBlock";
use crate::config::RpcType;
use crate::tx_senders::transaction::{TransactionConfig, build_transaction_with_config};
use crate::tx_senders::{TxResult, TxSender};
use anyhow::Context;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use tracing::info;

pub struct NextBlockTxSender {
    url: String,
    name: String,
    client: Client,
    tx_config: TransactionConfig,
    auth_token: Option<String>,
}

impl NextBlockTxSender {
    pub fn new(
        name: String,
        url: String,
        tx_config: TransactionConfig,
        client: Client,
        auth_token: Option<String>,
    ) -> Self {
        let sender = Self {
            url,
            name,
            tx_config,
            client,
            auth_token,
        };

        let test_client = sender.client.clone();
        let test_url = sender.url.clone();
        let test_auth = sender.auth_token.clone();
        let test_name = sender.name.clone();

        tokio::spawn(async move {
            info!("TEST {MODULE}: connection: {}", test_name);

            let mut request = test_client
                .post(&test_url)
                .timeout(std::time::Duration::from_secs(10))
                .header("Content-Type", "application/json")
                .body(r#"{"transaction":{"content":""}}"#);

            if let Some(auth) = &test_auth {
                request = request.header("Authorization", auth);
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    match status.as_u16() {
                        200 => {
                            info!("Status {MODULE}: {}", test_name);
                        }
                        _ => {
                            info!("Status {MODULE}: {} {}", test_name, status);
                        }
                    }
                }
                Err(e) => {
                    info!("Status Error: {test_name} {e}");
                }
            }
        });

        sender
    }
}

#[derive(Serialize)]
struct NextBlockRequest {
    transaction: String,
}

#[derive(Deserialize)]
struct NextBlockResponse {
    #[serde(rename = "txHash", alias = "tx_hash", alias = "transaction_id")]
    tx_hash: Option<String>,
    #[serde(alias = "signature")]
    signature: Option<String>,
    #[serde(alias = "message")]
    message: Option<String>,
}

#[async_trait]
impl TxSender for NextBlockTxSender {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn send_transaction(
        &self,
        index: u32,
        recent_blockhash: Hash,
        token_address: Pubkey,
        bonding_curve: Pubkey,
        associated_bonding_curve: Pubkey,
    ) -> anyhow::Result<TxResult> {
        let tx = build_transaction_with_config(
            &self.tx_config,
            &RpcType::NextBlock,
            recent_blockhash,
            token_address,
            bonding_curve,
            associated_bonding_curve,
        );

        let tx_bytes = bincode::serialize(&tx).context("{MODULE} Error: Failed to serialize transaction")?;

        let encoded_transaction = general_purpose::STANDARD.encode(&tx_bytes);

        let request_body = NextBlockRequest {
            transaction: encoded_transaction,
        };

        let json_body = serde_json::to_string(&request_body).context("{MODULE} Error: Failed to serialize request")?;

        let mut request_builder =
            self.client.post(&self.url).header("Content-Type", "application/json").body(json_body);

        if let Some(auth_token) = &self.auth_token {
            request_builder = request_builder.header("Authorization", auth_token);
        }

        let response = request_builder.send().await.context("{MODULE} Error: Failed to send request")?;

        let status = response.status();
        let response_text = response.text().await.context("{MODULE} Error: Failed to read response body")?;

        info!("Response: {MODULE} Status {} Response: {}", status, response_text);

        if status.is_success() {
            match serde_json::from_str::<NextBlockResponse>(&response_text) {
                Ok(response_data) => {
                    let tx_id = response_data
                        .tx_hash
                        .or(response_data.signature)
                        .unwrap_or_else(|| format!("nextblock_tx_{}", index));

                    info!("{MODULE} Success: TX ID: {}", tx_id);
                    Ok(TxResult::BundleID(tx_id))
                }
                Err(_) => {
                    info!("{MODULE} error: decode resp");
                    Ok(TxResult::BundleID(format!("nextblock_success_{}", index)))
                }
            }
        } else {
            let error_msg = format!("{MODULE} error: Status: {} Text: {}", status, response_text);
            info!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }
}
