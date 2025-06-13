use lazy_static::lazy_static;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub const METEORA_PROGRAM_ID: &str = "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB";
pub const WSOL_ADDRESS: &str = "So11111111111111111111111111111111111111112";

lazy_static! {
    pub static ref METEORA_ID: Pubkey = Pubkey::from_str(METEORA_PROGRAM_ID).unwrap();
    pub static ref WSOL_MINT: Pubkey = Pubkey::from_str(WSOL_ADDRESS).unwrap();
}

pub const RENT_ADDR: &str = "SysvarRent111111111111111111111111111111111";

pub const SYSTEM_PROGRAM_ADDR: &str = "11111111111111111111111111111111";
pub const TOKEN_PROGRAM_ADDR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

lazy_static! {
    pub static ref SYSTEM_PROGRAM_PUBKEY: Pubkey = Pubkey::from_str(SYSTEM_PROGRAM_ADDR).unwrap();
    pub static ref TOKEN_PROGRAM_PUBKEY: Pubkey = Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap();
}

pub const JITO_TIP_ADDR: &str = "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY";

// First from https://docs.nextblock.io/getting-started/quickstart#endpoints
pub const NEXTBLOCK_TIP_ADDR: &str = "NextbLoCkVtMGcV47JzewQdvBpLqT9TxQFozQkN98pE";

// https://docs.bloxroute.com/solana/trader-api/introduction/tip-and-tipping-addresses
pub const BLOXROUTE_TIP_ADDR: &str = "HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY";

lazy_static! {
    pub static ref JITO_TIP_PUBKEY: Pubkey = Pubkey::from_str(JITO_TIP_ADDR).unwrap();
    pub static ref NEXTBLOCK_TIP_PUBKEY: Pubkey = Pubkey::from_str(NEXTBLOCK_TIP_ADDR).unwrap();
    pub static ref BLOXROUTE_TIP_PUBKEY: Pubkey = Pubkey::from_str(BLOXROUTE_TIP_ADDR).unwrap();
}
