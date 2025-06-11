use crate::config::RpcType;
use crate::tx_senders::transaction::{TransactionConfig, PoolVaultInfo, build_transaction_with_config};
use crate::tx_senders::{TxResult, TxSender};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use solana_sdk::bs58;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;
use tracing::debug;

pub struct JitoTxSender {
    url: String,
    name: String,
    client: Client,
    tx_config: TransactionConfig,
}

impl JitoTxSender {
    pub fn new(name: String, url: String, tx_config: TransactionConfig, client: Client) -> Self {
        Self {
            url,
            name,
            tx_config,
            client,
        }
    }

    pub fn build_transaction_with_config(
        &self,
        _index: u32,
        recent_blockhash: Hash,
        target_token: Pubkey,
        pool_vault_info: PoolVaultInfo,
    ) -> VersionedTransaction {
        build_transaction_with_config(
            &self.tx_config,
            &RpcType::Jito,
            recent_blockhash,
            target_token,
            pool_vault_info,
        )
    }
}

#[derive(Deserialize)]
pub struct JitoResponse {
    pub result: String,
}

#[async_trait]
impl TxSender for JitoTxSender {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn send_transaction(
        &self,
        index: u32,
        recent_blockhash: Hash,
        target_token: Pubkey,
        pool_vault_info: PoolVaultInfo,
    ) -> anyhow::Result<TxResult> {
        let tx = self.build_transaction_with_config(
            index,
            recent_blockhash,
            target_token,
            pool_vault_info,
        );
        let tx_bytes = bincode::serialize(&tx).context("cannot serialize tx to bincode")?;
        let encoded_transaction = bs58::encode(tx_bytes).into_string();
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [encoded_transaction]
        });
        debug!("sending tx: {}", body.to_string());
        let response = self.client.post(&self.url).json(&body).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("failed to send tx: {}", body));
        }
        let parsed_resp = serde_json::from_str::<JitoResponse>(&body).context("cannot deserialize signature")?;
        Ok(TxResult::BundleID(parsed_resp.result))
    }
}