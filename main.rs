use crate::bench::Bench;
use crate::config::PingThingsArgs;
use crate::geyser::{GeyserResult, YellowstoneGrpcGeyser, YellowstoneGrpcGeyserClient};
use crate::meteora::MeteoraController;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequestFilterAccounts, SubscribeRequestFilterTransactions,
};

pub const METEORA_PROGRAM_ID: Pubkey = pubkey!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");

mod bench;
mod config;
mod core;
mod geyser;
mod meteora;
mod tx_senders;

#[tokio::main]
pub async fn main() -> GeyserResult<()> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!("Starting bot");

    dotenv::dotenv().ok();

    let config_controller: PingThingsArgs = PingThingsArgs::new();

    let bench_controller: Bench = Bench::new(config_controller.clone());

    let meteora_controller: MeteoraController =
        MeteoraController::new(config_controller.clone(), bench_controller.clone());

    let account_filters: HashMap<String, SubscribeRequestFilterAccounts> = HashMap::new();

    let transaction_filter = SubscribeRequestFilterTransactions {
        vote: Some(false),
        failed: Some(false),
        account_include: vec![METEORA_PROGRAM_ID.to_string()],
        account_exclude: vec![],
        account_required: vec![],
        signature: None,
    };

    let mut transaction_filters: HashMap<String, SubscribeRequestFilterTransactions> = HashMap::new();
    transaction_filters.insert("meteora_transaction_filter".to_string(), transaction_filter);

    let geyser_url = config_controller.geyser_url.clone();
    let yellowstone_grpc = YellowstoneGrpcGeyserClient::new(
        config_controller.geyser_url,
        Some(config_controller.geyser_x_token),
        Some(CommitmentLevel::Processed),
        account_filters,
        transaction_filters,
        Arc::new(RwLock::new(HashSet::new())),
    );

    info!("Geyser endpoint: {}", geyser_url);

    let result = yellowstone_grpc.consume(meteora_controller).await;

    match result {
        Ok(_) => {
            info!("Monitoring success");
        }
        Err(e) => {
            error!("Monitoring failed: {:?}", e);
        }
    }

    Ok(())
}
