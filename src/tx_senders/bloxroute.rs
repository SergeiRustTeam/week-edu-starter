//! Unified sender for Bloxroute *and* NextBlock.
//! API spec: bloXroute Trader-API & NextBlock `/api/v2/submit`  :contentReference[oaicite:0]{index=0}

use anyhow::Context;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use solana_sdk::{hash::Hash, pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction};
use std::str::FromStr;
use tracing::{debug, error, info};

use crate::config::RpcType;
use crate::meteora::SwapAccountsFromPoolCreationInstruction;
use crate::tx_senders::transaction::{TransactionConfig, build_meteora_swap_transaction_with_config};
use crate::tx_senders::{TxResult, TxSender};

/// One struct, many relays (Bloxroute, NextBlock)
pub struct RelayTxSender {
    url: String,
    name: String,
    auth_header: String,
    rpc_type: RpcType,
    client: Client,
    tx_config: TransactionConfig,
}

impl RelayTxSender {
    pub fn new(
        name: String,
        url: String,
        auth_header: String,
        rpc_type: RpcType,
        tx_config: TransactionConfig,
        client: Client,
    ) -> Self {
        Self {
            url,
            name,
            auth_header,
            rpc_type,
            client,
            tx_config,
        }
    }
}

#[derive(Deserialize)]
struct RelayResponse {
    signature: String,
    #[serde(default)]
    uuid: Option<String>, // present in NextBlock
}

#[async_trait]
impl TxSender for RelayTxSender {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn send_meteora_swap_transaction(
        &self,
        recent_blockhash: Hash,
        token_source_mint: Pubkey, // always WSOL
        token_dest_mint: Pubkey,
        protocol_token_fee: Pubkey,
        accounts: SwapAccountsFromPoolCreationInstruction,
    ) -> anyhow::Result<TxResult> {
        let transaction: VersionedTransaction = build_meteora_swap_transaction_with_config(
            &self.tx_config,
            &self.rpc_type,
            recent_blockhash,
            token_source_mint, // always WSOL
            token_dest_mint,
            protocol_token_fee,
            accounts,
        );

        // Base-64 encode.
        let encoded_tx = general_purpose::STANDARD.encode(bincode::serialize(&transaction).context("serialize tx")?);

        let body = json!({ "transaction": { "content": encoded_tx } });

        debug!("{:?} submit body: {}", self.rpc_type, body);

        // 4. POST
        let request = self.client.post(&self.url).header("Authorization", &self.auth_header).json(&body);

        debug!("request={:?}", request);

        let res = request.send().await.with_context(|| format!("HTTP to {:?} failed", self.rpc_type))?;

        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            error!("rpc failed: {}, text={}", self.name(), text);
            return Err(anyhow::anyhow!("{:?} error {}: {}", self.rpc_type, status, text));
        }

        // Parse signature
        let parsed: RelayResponse = serde_json::from_str(&text).context("decode relay response")?;

        let sig = Signature::from_str(&parsed.signature)?;
        Ok(TxResult::Signature(sig))
    }
}
