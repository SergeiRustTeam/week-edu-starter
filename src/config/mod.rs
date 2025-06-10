use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct PingThingsArgs {
    // rpc_name -> rpc_url
    pub rpc: HashMap<String, RpcConfig>,
    pub http_rpc: String,
    pub ws_rpc: String,
    pub geyser_url: String,
    pub geyser_x_token: String,
    pub private_key: String,
    pub compute_unit_price: u64,
    pub compute_unit_limit: u32,
    pub tip: f64,
    pub buy_amount: f64,
    pub min_amount_out: f64,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RpcType {
    #[default]
    SolanaRpc,
    Jito,
    Bloxroute,
    NextBlock,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RpcConfig {
    pub url: String,
    #[serde(default)]
    pub auth: Option<String>,
    #[serde(default)]
    pub rpc_type: RpcType,
}

impl PingThingsArgs {
    pub fn new() -> Self {
        let config_yaml = fs::read_to_string("./config.yaml").expect("cannot find config file");
        serde_yaml::from_str::<PingThingsArgs>(&config_yaml).expect("invalid config file")
    }
}
