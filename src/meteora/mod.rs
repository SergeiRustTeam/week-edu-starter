use crate::bench::Bench;
use crate::config::PingThingsArgs;
use crate::core::extract_instructions;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionStatusMeta;
use std::str::FromStr;
use tracing::info;

pub const METEORA_PROGRAM_ADDR: &str = "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB";
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const IX_DISCRIMINATOR_SIZE: usize = 8;

pub const INIT_POOL_CONFIG2_DISC: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_address: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub a_vault: Pubkey,
    pub b_vault: Pubkey,
    pub a_token_vault: Pubkey,
    pub b_token_vault: Pubkey,
    pub a_vault_lp: Pubkey,
    pub b_vault_lp: Pubkey,
    pub is_wsol_pair: bool,
    pub wsol_is_token_a: bool,
    pub instruction_type: String,
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
    is_buy: bool,
    processed_pools: std::collections::HashSet<Pubkey>,
}

impl MeteoraController {
    pub fn new(config: PingThingsArgs, bench: Bench) -> Self {
        MeteoraController {
            config,
            bench,
            is_buy: false,
            processed_pools: std::collections::HashSet::new(),
        }
    }

    pub async fn transaction_handler(
        &mut self,
        signature: Signature,
        transaction: VersionedTransaction,
        meta: TransactionStatusMeta,
        _is_vote: bool,
        _slot: u64,
    ) -> anyhow::Result<()> {
        let instructions: Vec<solana_sdk::instruction::Instruction> = extract_instructions(meta, transaction.clone())?;

        if !self.is_buy {
            for instruction in instructions.iter() {
                if instruction.program_id == Pubkey::from_str(METEORA_PROGRAM_ADDR)? {
                    if let Ok(pool_info) = self.process_meteora_instruction(&instruction).await {
                        if pool_info.is_wsol_pair
                            && pool_info.liquidity_added
                            && !self.processed_pools.contains(&pool_info.pool_address)
                        {
                            info!("Meteota: new wsol pool found: {}", pool_info.pool_address);

                            let target_token = if pool_info.wsol_is_token_a {
                                pool_info.token_b_mint
                            } else {
                                pool_info.token_a_mint
                            };
                            self.processed_pools.insert(pool_info.pool_address);

                            let recent_blockhash: Hash = *transaction.message.recent_blockhash();

                            self.bench
                                .clone()
                                .send_meteora_buy_tx(
                                    recent_blockhash,
                                    target_token,
                                    pool_info.pool_address,
                                    pool_info.a_vault,
                                )
                                .await;
                        }
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
            _ => Err(anyhow::anyhow!("Not the target instruction type")),
        }
    }

    async fn parse_init_pool_config2(
        &self,
        instruction: &solana_sdk::instruction::Instruction,
    ) -> anyhow::Result<PoolInfo> {
        if instruction.accounts.len() < 20 {
            return Err(anyhow::anyhow!("Not enough accounts for Meteora pool creation"));
        }

        let pool_address = instruction.accounts[0].pubkey;
        let token_a_mint = instruction.accounts[3].pubkey;
        let token_b_mint = instruction.accounts[4].pubkey;
        let a_vault = instruction.accounts[5].pubkey;
        let b_vault = instruction.accounts[6].pubkey;
        let a_token_vault = instruction.accounts[7].pubkey;
        let b_token_vault = instruction.accounts[8].pubkey;
        let a_vault_lp = instruction.accounts[11].pubkey;
        let b_vault_lp = instruction.accounts[12].pubkey;

        let wsol_pubkey = Pubkey::from_str(WSOL_MINT)?;
        let is_wsol_pair = token_a_mint == wsol_pubkey || token_b_mint == wsol_pubkey;
        let wsol_is_token_a = token_a_mint == wsol_pubkey;

        let mut has_liquidity = true;
        if instruction.data.len() > IX_DISCRIMINATOR_SIZE {
            let ix_data = &instruction.data[IX_DISCRIMINATOR_SIZE..];
            if let Ok(init_data) = InitPoolConfig2Data::try_from_slice(ix_data) {
                has_liquidity = init_data.token_a_amount > 0 && init_data.token_b_amount > 0;
            }
        }

        Ok(PoolInfo {
            pool_address,
            token_a_mint,
            token_b_mint,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp,
            b_vault_lp,
            is_wsol_pair,
            wsol_is_token_a,
            instruction_type: "initializePermissionlessConstantProductPoolWithConfig2".to_string(),
            liquidity_added: has_liquidity,
        })
    }
}
