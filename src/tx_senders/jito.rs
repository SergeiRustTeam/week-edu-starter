use crate::config::RpcType;
use crate::meteora::SwapAccountsFromPoolCreationInstruction;
use crate::tx_senders::transaction::{TransactionConfig, build_meteora_swap_transaction_with_config};
use crate::tx_senders::{TxResult, TxSender};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};
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
}

#[derive(Deserialize)]
pub struct JitoBundleStatusResponseInnerContext {
    pub slot: u64,
}

#[derive(Deserialize)]
pub struct JitoBundleStatusResponseInnerValue {
    pub slot: u64,
    pub bundle_id: String,
    pub transactions: Vec<String>,
    pub confirmation_status: String,
    pub err: Value,
}

#[derive(Deserialize)]
pub struct JitoBundleStatusResponseInner {
    pub context: JitoBundleStatusResponseInnerContext,
    pub value: Vec<JitoBundleStatusResponseInnerValue>,
}
#[derive(Deserialize)]
pub struct JitoBundleStatusResponse {
    pub result: JitoBundleStatusResponseInner,
}

#[derive(Deserialize)]
pub struct JitoResponse {
    //bundle id is response
    pub result: String,
}

#[async_trait]
impl TxSender for JitoTxSender {
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
        let transaction = build_meteora_swap_transaction_with_config(
            &self.tx_config,
            &RpcType::Jito,
            recent_blockhash,
            token_source_mint, // always WSOL
            token_dest_mint,
            protocol_token_fee,
            accounts,
        );

        let tx_bytes = bincode::serialize(&transaction).context("cannot serialize transaction to bincode")?;

        let encoded_transaction = bs58::encode(tx_bytes).into_string();

        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [encoded_transaction]
        });

        debug!("sending meteora swap tx: {}", body);

        let response = self.client.post(&self.url).json(&body).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!("Jito failed to send tx: {}", body));
        }

        let parsed_resp = serde_json::from_str::<JitoResponse>(&body).context("cannot deserialize Jito response")?;

        Ok(TxResult::BundleID(parsed_resp.result))
    }
}
