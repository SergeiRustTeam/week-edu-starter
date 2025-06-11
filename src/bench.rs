use crate::config::PingThingsArgs;
use crate::tx_senders::transaction::{TransactionConfig, PoolVaultInfo};
use crate::tx_senders::{create_tx_sender, TxSender};
use reqwest::Client;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct Bench {
    rpcs: Vec<Arc<dyn TxSender>>,
}

impl Bench {
    pub fn new(config: PingThingsArgs) -> Self {
        let tx_config: TransactionConfig = config.clone().into();
        let client = Client::new();

        let rpcs = config
            .rpc
            .clone()
            .into_iter()
            .map(|(name, rpc)| create_tx_sender(name, rpc, tx_config.clone(), client.clone()))
            .collect::<Vec<Arc<dyn TxSender>>>();

        Bench { rpcs }
    }

    pub async fn send_meteora_buy_tx(
        self,
        recent_blockhash: Hash,
        target_token: Pubkey,
        pool_vault_info: PoolVaultInfo,
    ) {
        tokio::select! {
            _ = self.send_meteora_buy_tx_inner(
                recent_blockhash,
                target_token,
                pool_vault_info,
            ) => {}
        }
    }

    async fn send_meteora_buy_tx_inner(
        self,
        recent_blockhash: Hash,
        target_token: Pubkey,
        pool_vault_info: PoolVaultInfo,
    ) {
        let start = tokio::time::Instant::now();

        let mut tx_handles = Vec::new();
        for rpc in &self.rpcs {
            let rpc_sender = rpc.clone();
            let pool_vault_info_clone = pool_vault_info.clone();
            let hdl = tokio::spawn(async move {
                let index = 0;
                if let Err(e) = rpc_sender
                    .send_transaction(
                        index,
                        recent_blockhash,
                        target_token,
                        pool_vault_info_clone,
                    )
                    .await
                {
                    info!("Meteora: transaction error: {:?}", e);
                } else {
                    info!("Meteora: transaction success");
                }
            });
            tx_handles.push(hdl);
        }

        for hdl in tx_handles {
            hdl.await.unwrap_or_default();
        }

        info!(
            "Meteora buy completed in {:?} ms",
            start.elapsed().as_millis() as u64
        );
    }
}