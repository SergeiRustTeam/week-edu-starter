use crate::bench::Bench;
use crate::config::PingThingsArgs;
use crate::core::extract_instructions;
use crate::tx_senders::constants::{
    ADD_LIQUIDITY_DISC, DEPOSIT_DISC, INIT_POOL_CONFIG2_DISC, IX_DISCRIMINATOR_SIZE, METEORA_PROGRAM_ADDR, WSOL_MINT,
};
use crate::tx_senders::transaction::PoolVaultInfo;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionStatusMeta;
use std::str::FromStr;
use tracing::info;

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_address: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub a_vault: Pubkey,
    pub b_vault: Pubkey,
    pub a_token_vault: Pubkey,
    pub b_token_vault: Pubkey,
    pub a_vault_lp_mint: Pubkey,
    pub b_vault_lp_mint: Pubkey,
    pub a_vault_lp: Pubkey,
    pub b_vault_lp: Pubkey,
    pub protocol_token_a_fee: Pubkey,
    pub protocol_token_b_fee: Pubkey,
    pub is_wsol_pair: bool,
    pub wsol_is_token_a: bool,
    pub liquidity_added: bool,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct InitPoolConfig2Data {
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub activation_point: Option<u64>,
}

pub struct MeteoraController {
    config: PingThingsArgs,
    bench: Bench,
    processed_pools: std::collections::HashSet<Pubkey>,
}

impl MeteoraController {
    pub fn new(config: PingThingsArgs, bench: Bench) -> Self {
        MeteoraController {
            config,
            bench,
            processed_pools: std::collections::HashSet::new(),
        }
    }

    pub async fn transaction_handler(
        &mut self,
        _signature: Signature,
        transaction: VersionedTransaction,
        meta: TransactionStatusMeta,
        _slot: u64,
    ) -> anyhow::Result<()> {
        let instructions: Vec<solana_sdk::instruction::Instruction> = extract_instructions(meta, transaction.clone())?;

        for instruction in instructions.iter() {
            if instruction.program_id == Pubkey::from_str(METEORA_PROGRAM_ADDR)? {
                if let Ok(pool_info) = self.process_meteora_instruction(&instruction).await {
                    if pool_info.is_wsol_pair
                        && pool_info.liquidity_added
                        && !self.processed_pools.contains(&pool_info.pool_address)
                    {
                        info!("METEORA: Liquidity added to WSOL pool");

                        let target_token = if pool_info.wsol_is_token_a {
                            pool_info.token_b_mint
                        } else {
                            pool_info.token_a_mint
                        };

                        self.processed_pools.insert(pool_info.pool_address);

                        let recent_blockhash: Hash = *transaction.message.recent_blockhash();

                        let pool_vault_info = PoolVaultInfo {
                            pool_address: pool_info.pool_address,
                            token_a_mint: pool_info.token_a_mint,
                            token_b_mint: pool_info.token_b_mint,
                            a_vault: pool_info.a_vault,
                            b_vault: pool_info.b_vault,
                            a_token_vault: pool_info.a_token_vault,
                            b_token_vault: pool_info.b_token_vault,
                            a_vault_lp_mint: pool_info.a_vault_lp_mint,
                            b_vault_lp_mint: pool_info.b_vault_lp_mint,
                            a_vault_lp: pool_info.a_vault_lp,
                            b_vault_lp: pool_info.b_vault_lp,
                            protocol_token_a_fee: pool_info.protocol_token_a_fee,
                            protocol_token_b_fee: pool_info.protocol_token_b_fee,
                            wsol_is_token_a: pool_info.wsol_is_token_a,
                        };

                        self.bench.clone().send_meteora_buy_tx(recent_blockhash, target_token, pool_vault_info).await;

                        info!("Meteora: buy transactions completed");
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_meteora_instruction(
        &self,
        instruction: &solana_sdk::instruction::Instruction,
    ) -> anyhow::Result<PoolInfo> {
        if instruction.data.len() < IX_DISCRIMINATOR_SIZE {
            return Err(anyhow::anyhow!("Instruction data too short"));
        }

        let ix_discriminator: [u8; 8] = instruction.data[0..IX_DISCRIMINATOR_SIZE].try_into()?;

        match ix_discriminator {
            INIT_POOL_CONFIG2_DISC => self.parse_init_pool_config2(instruction).await,
            ADD_LIQUIDITY_DISC => self.parse_add_liquidity(instruction).await,
            DEPOSIT_DISC => self.parse_deposit(instruction).await,
            _ => Err(anyhow::anyhow!("Not a target instruction type")),
        }
    }

    async fn parse_init_pool_config2(
        &self,
        instruction: &solana_sdk::instruction::Instruction,
    ) -> anyhow::Result<PoolInfo> {
        if instruction.accounts.len() < 18 {
            return Err(anyhow::anyhow!("Not enough accounts for Meteora pool creation"));
        }

        let pool_address = instruction.accounts[0].pubkey;
        let token_a_mint = instruction.accounts[3].pubkey;
        let token_b_mint = instruction.accounts[4].pubkey;
        let a_vault = instruction.accounts[5].pubkey;
        let b_vault = instruction.accounts[6].pubkey;
        let a_token_vault = instruction.accounts[7].pubkey;
        let b_token_vault = instruction.accounts[8].pubkey;
        let a_vault_lp_mint = instruction.accounts[9].pubkey;
        let b_vault_lp_mint = instruction.accounts[10].pubkey;
        let a_vault_lp = instruction.accounts[11].pubkey;
        let b_vault_lp = instruction.accounts[12].pubkey;
        let protocol_token_a_fee = instruction.accounts[16].pubkey;
        let protocol_token_b_fee = instruction.accounts[17].pubkey;

        let wsol_pubkey = Pubkey::from_str(WSOL_MINT)?;
        let is_wsol_pair = token_a_mint == wsol_pubkey || token_b_mint == wsol_pubkey;
        let wsol_is_token_a = token_a_mint == wsol_pubkey;

        let mut has_liquidity = true;
        if instruction.data.len() > IX_DISCRIMINATOR_SIZE {
            let ix_data = &instruction.data[IX_DISCRIMINATOR_SIZE..];
            if let Ok(init_data) = InitPoolConfig2Data::try_from_slice(ix_data) {
                has_liquidity = init_data.token_a_amount > 0 && init_data.token_b_amount > 0;
                info!(
                    "Pool liquidity check: token_a={}, token_b={}",
                    init_data.token_a_amount, init_data.token_b_amount
                );
            }
        }

        if is_wsol_pair {
            info!("WSOL pool detected: {} (liquidity: {})", pool_address, has_liquidity);
        }

        Ok(PoolInfo {
            pool_address,
            token_a_mint,
            token_b_mint,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint,
            b_vault_lp_mint,
            a_vault_lp,
            b_vault_lp,
            protocol_token_a_fee,
            protocol_token_b_fee,
            is_wsol_pair,
            wsol_is_token_a,
            liquidity_added: has_liquidity,
        })
    }

    async fn parse_add_liquidity(
        &self,
        instruction: &solana_sdk::instruction::Instruction,
    ) -> anyhow::Result<PoolInfo> {
        if instruction.accounts.len() < 10 {
            return Err(anyhow::anyhow!("Not enough accounts for add liquidity"));
        }

        let pool_address = instruction.accounts[0].pubkey;

        info!("Meteora: ADD_LIQUIDITY detected for pool: {}", pool_address);

        Ok(PoolInfo {
            pool_address,
            token_a_mint: Pubkey::default(),
            token_b_mint: Pubkey::default(),
            a_vault: Pubkey::default(),
            b_vault: Pubkey::default(),
            a_token_vault: Pubkey::default(),
            b_token_vault: Pubkey::default(),
            a_vault_lp_mint: Pubkey::default(),
            b_vault_lp_mint: Pubkey::default(),
            a_vault_lp: Pubkey::default(),
            b_vault_lp: Pubkey::default(),
            protocol_token_a_fee: Pubkey::default(),
            protocol_token_b_fee: Pubkey::default(),
            is_wsol_pair: true,
            wsol_is_token_a: false,
            liquidity_added: true,
        })
    }

    async fn parse_deposit(&self, instruction: &solana_sdk::instruction::Instruction) -> anyhow::Result<PoolInfo> {
        if instruction.accounts.len() < 8 {
            return Err(anyhow::anyhow!("Not enough accounts for deposit"));
        }

        let pool_address = instruction.accounts[0].pubkey;

        info!("Meteora: DEP detected for pool: {}", pool_address);

        Ok(PoolInfo {
            pool_address,
            token_a_mint: Pubkey::default(),
            token_b_mint: Pubkey::default(),
            a_vault: Pubkey::default(),
            b_vault: Pubkey::default(),
            a_token_vault: Pubkey::default(),
            b_token_vault: Pubkey::default(),
            a_vault_lp_mint: Pubkey::default(),
            b_vault_lp_mint: Pubkey::default(),
            a_vault_lp: Pubkey::default(),
            b_vault_lp: Pubkey::default(),
            protocol_token_a_fee: Pubkey::default(),
            protocol_token_b_fee: Pubkey::default(),
            is_wsol_pair: true,
            wsol_is_token_a: false,
            liquidity_added: true,
        })
    }
}
