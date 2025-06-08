use crate::config::PingThingsArgs;
use crate::meteora::SwapAccountsFromPoolCreationInstruction;
use crate::tx_senders::solana_rpc::TxMetrics;
use crate::tx_senders::transaction::TransactionConfig;
use crate::tx_senders::{TxSender, create_tx_sender};
use reqwest::Client;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::{error, info};

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

    pub async fn send_and_confirm_swap_transaction(
        tx_index: u32,
        rpc_sender: Arc<dyn TxSender>,
        recent_blockhash: Hash,
        token_source_mint: Pubkey, // always WSOL
        token_dest_mint: Pubkey,
        protocol_token_fee: Pubkey,
        accounts: SwapAccountsFromPoolCreationInstruction,
    ) -> anyhow::Result<()> {
        let started_at = tokio::time::Instant::now();

        let tx_result = rpc_sender
            .send_meteora_swap_transaction(
                recent_blockhash,
                token_source_mint, // always WSOL
                token_dest_mint,
                protocol_token_fee,
                accounts,
            )
            .await;

        let rpc_duration = started_at.elapsed().as_millis() as u64;
        match tx_result {
            Ok(_) => {
                info!(
                    "successfully completed rpc: {:?} {:?} ms",
                    rpc_sender.name(),
                    rpc_duration
                );
            }
            Err(_) => {
                error!("failed rpc: {:?} {:?} ms", rpc_sender.name(), rpc_duration);
            }
        };

        Ok(())
    }

    pub async fn send_swap_tx(
        self,
        recent_blockhash: Hash,
        token_source_mint: Pubkey,
        token_dest_mint: Pubkey,
        protocol_token_fee: Pubkey,
        accounts: SwapAccountsFromPoolCreationInstruction,
    ) -> anyhow::Result<()> {
        let start = tokio::time::Instant::now();
        info!("starting create swap tx");
        let mut tx_handles = Vec::new();

        for rpc in &self.rpcs {
            info!("Creating sender: {}", rpc.name());
            let rpc_sender = rpc.clone();
            let accounts = accounts.clone();
            let hdl = tokio::spawn(async move {
                let index = 0;
                if let Err(e) = Self::send_and_confirm_swap_transaction(
                    index,
                    rpc_sender,
                    recent_blockhash,
                    token_source_mint, // always WSOL
                    token_dest_mint,
                    protocol_token_fee,
                    accounts,
                )
                .await
                {
                    error!("error send_and_confirm_swap_transaction {:?}", e);
                }
            });
            info!("Task hanled: {:?}", hdl);
            tx_handles.push(hdl);
        }

        info!("waiting for swap transactions to complete...");

        for hdl in tx_handles {
            hdl.await.unwrap_or_default();
        }

        info!("swap bench complete! {:?} ms", start.elapsed().as_millis() as u64);

        Ok(())
    }
}
