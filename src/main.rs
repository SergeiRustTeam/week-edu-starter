use crate::bench::Bench;
use crate::config::PingThingsArgs;
use crate::geyser::{GeyserResult, YellowstoneGrpcGeyser, YellowstoneGrpcGeyserClient};
use crate::pumpfun::PumpFunController;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequestFilterAccounts, SubscribeRequestFilterTransactions,
};

pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
mod bench;
mod config;
mod core;
mod geyser;
mod pumpfun;
mod tx_senders;

#[tokio::main]
pub async fn main() -> GeyserResult<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .unwrap();

    let config_controller: PingThingsArgs = PingThingsArgs::new();
    let bench_controller: Bench = Bench::new(config_controller.clone());

    let pumpfun_controller: PumpFunController =
        PumpFunController::new(config_controller.clone(), bench_controller.clone());

    info!("starting with config {:?}", config_controller);

    env_logger::init();
    dotenv::dotenv().ok();

    let account_filters: HashMap<String, SubscribeRequestFilterAccounts> = HashMap::new();

    let transaction_filter = SubscribeRequestFilterTransactions {
        vote: Some(false),
        failed: Some(false),
        account_include: vec![PUMPFUN_PROGRAM_ID.to_string().clone()],
        account_exclude: vec![],
        account_required: vec![],
        signature: None,
    };

    let mut transaction_filters: HashMap<String, SubscribeRequestFilterTransactions> = HashMap::new();

    transaction_filters.insert("pumpfun_transaction_filter".to_string(), transaction_filter);

    let yellowstone_grpc = YellowstoneGrpcGeyserClient::new(
        config_controller.geyser_url,
        Some(config_controller.geyser_x_token),
        Some(CommitmentLevel::Processed),
        account_filters,
        transaction_filters,
        Arc::new(RwLock::new(HashSet::new())),
    );

    let _ = yellowstone_grpc.consume(pumpfun_controller).await;
    Ok(())
}
