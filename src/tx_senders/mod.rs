use crate::config::{RpcConfig, RpcType};
use crate::meteora::*;
use crate::tx_senders::bloxroute::RelayTxSender;
use crate::tx_senders::jito::JitoTxSender;
use crate::tx_senders::solana_rpc::GenericRpc;
use crate::tx_senders::transaction::TransactionConfig;
use async_trait::async_trait;
use reqwest::Client;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::sync::Arc;
use tracing::info;

pub mod bloxroute;
pub mod constants;
pub mod jito;
pub mod solana_rpc;
pub mod transaction;

#[derive(Debug, Clone)]
pub enum TxResult {
    Signature(Signature),
    BundleID(String),
}

impl Into<String> for TxResult {
    fn into(self) -> String {
        match self {
            TxResult::Signature(sig) => sig.to_string(),
            TxResult::BundleID(bundle_id) => bundle_id,
        }
    }
}

#[async_trait]
pub trait TxSender: Sync + Send {
    fn name(&self) -> String;

    async fn send_meteora_swap_transaction(
        &self,
        recent_blockhash: Hash,
        token_a_mint: Pubkey, // always WSOL
        token_b_mint: Pubkey,
        protocol_token_fee: Pubkey,
        accounts: SwapAccountsFromPoolCreationInstruction,
    ) -> anyhow::Result<TxResult>;
}

pub fn create_tx_sender(
    name: String,
    rpc_config: RpcConfig,
    tx_config: TransactionConfig,
    client: Client,
) -> Arc<dyn TxSender> {
    info!("create_tx_sender {:?}", rpc_config.rpc_type);
    match rpc_config.rpc_type {
        RpcType::SolanaRpc => {
            let tx_sender = GenericRpc::new(name, rpc_config.url, tx_config, RpcType::SolanaRpc);
            Arc::new(tx_sender)
        }
        RpcType::Jito => {
            let tx_sender = JitoTxSender::new(name, rpc_config.url, tx_config, client);
            Arc::new(tx_sender)
        }
        RpcType::Bloxroute | RpcType::NextBlock => {
            let tx_sender = RelayTxSender::new(
                name,
                rpc_config.url.clone(),
                rpc_config.auth.expect("auth header required"),
                rpc_config.rpc_type,
                tx_config,
                client,
            );
            Arc::new(tx_sender)
        }
    }
}
