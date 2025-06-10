use crate::config::PingThingsArgs;
use crate::tx_senders::solana_rpc::TxMetrics;
use crate::tx_senders::transaction::TransactionConfig;
use crate::tx_senders::{TxSender, create_tx_sender};
use reqwest::Client;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct Bench {
    config: PingThingsArgs,
    tx_subscribe_sender: tokio::sync::mpsc::Sender<TxMetrics>,
    rpcs: Vec<Arc<dyn TxSender>>,
    client: Client,
}

impl Bench {
    pub fn new(config: PingThingsArgs) -> Self {
        let (tx_subscribe_sender, _tx_subscribe_receiver) = tokio::sync::mpsc::channel(100);
        let tx_config: TransactionConfig = config.clone().into();
        let client = Client::new();

        let rpcs = config
            .rpc
            .clone()
            .into_iter()
            .map(|(name, rpc)| create_tx_sender(name, rpc, tx_config.clone(), client.clone()))
            .collect::<Vec<Arc<dyn TxSender>>>();

        Bench {
            config,
            tx_subscribe_sender,
            rpcs,
            client,
        }
    }

    pub async fn send_and_confirm_transaction(
        tx_index: u32,
        rpc_sender: Arc<dyn TxSender>,
        recent_blockhash: Hash,
        token_address: Pubkey,
        bonding_curve: Pubkey,
        associated_bonding_curve: Pubkey,
    ) -> anyhow::Result<()> {
        let _tx_result = rpc_sender
            .send_transaction(
                tx_index,
                recent_blockhash,
                token_address,
                bonding_curve,
                associated_bonding_curve,
            )
            .await?;
        Ok(())
    }

    pub async fn send_meteora_buy_tx(
        self,
        recent_blockhash: Hash,
        token_address: Pubkey,
        pool_address: Pubkey,
        vault_address: Pubkey,
    ) {
        tokio::select! {
            _ = self.send_meteora_buy_tx_inner(
                recent_blockhash,
                token_address,
                pool_address,
                vault_address,
            ) => {}
        }
    }

    async fn send_meteora_buy_tx_inner(
        self,
        recent_blockhash: Hash,
        token_address: Pubkey,
        pool_address: Pubkey,
        vault_address: Pubkey,
    ) {
        let start = tokio::time::Instant::now();

        let mut tx_handles = Vec::new();
        for rpc in &self.rpcs {
            let rpc_sender = rpc.clone();
            let hdl = tokio::spawn(async move {
                let index = 0;
                if let Err(e) = Self::send_and_confirm_transaction(
                    index,
                    rpc_sender,
                    recent_blockhash,
                    token_address,
                    pool_address,
                    vault_address,
                )
                .await
                {
                    info!("Meteora: transaction error: {:?}", e);
                } else {
                    info!("Meteora: transaction success via");
                }
            });
            tx_handles.push(hdl);
        }

        for hdl in tx_handles {
            hdl.await.unwrap_or_default();
        }

        info!("Meteora buy success {:?} ms", start.elapsed().as_millis() as u64);
    }
}
