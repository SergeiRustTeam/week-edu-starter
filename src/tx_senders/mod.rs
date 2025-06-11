use crate::config::{RpcConfig, RpcType};
use crate::tx_senders::bloxroute::BloxrouteTxSender;
use crate::tx_senders::jito::JitoTxSender;
use crate::tx_senders::nextblock::NextBlockTxSender;
use crate::tx_senders::solana_rpc::GenericRpc;
use crate::tx_senders::transaction::{TransactionConfig, PoolVaultInfo};
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
pub mod nextblock;
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
    async fn send_transaction(
        &self,
        index: u32,
        recent_blockhash: Hash,
        target_token: Pubkey,
        pool_vault_info: PoolVaultInfo,
    ) -> anyhow::Result<TxResult>;
}

pub fn create_tx_sender(
    name: String,
    rpc_config: RpcConfig,
    tx_config: TransactionConfig,
    client: Client,
) -> Arc<dyn TxSender> {
    match rpc_config.rpc_type {
        RpcType::SolanaRpc => {
            info!("Creating: Solana RPC-sender: {}", rpc_config.url);
            let tx_sender = GenericRpc::new(name, rpc_config.url, tx_config, RpcType::SolanaRpc);
            Arc::new(tx_sender)
        }
        RpcType::Jito => {
            info!("Creating: Jito-sender: {}", rpc_config.url);
            let tx_sender = JitoTxSender::new(name, rpc_config.url, tx_config, client);
            Arc::new(tx_sender)
        }
        RpcType::Bloxroute => {
            info!("Creating: Bloxroute-sender: {}", rpc_config.url);
            let tx_sender = BloxrouteTxSender::new(name, rpc_config.url, tx_config, client, rpc_config.auth);
            Arc::new(tx_sender)
        }
        RpcType::NextBlock => {
            info!("Creating: NextBlock-sender: {}", rpc_config.url);
            let tx_sender = NextBlockTxSender::new(name, rpc_config.url, tx_config, client, rpc_config.auth);
            Arc::new(tx_sender)
        }
    }
}