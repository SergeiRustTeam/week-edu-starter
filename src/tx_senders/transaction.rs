use crate::config::{PingThingsArgs, RpcType};
use crate::tx_senders::constants::{JITO_TIP_ADDR, METEORA_PROGRAM_ADDR, TOKEN_PROGRAM_ADDR, WSOL_MINT};

use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::VersionedMessage;
use solana_sdk::message::v0::Message;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction;
use solana_sdk::transaction::VersionedTransaction;
use spl_associated_token_account::get_associated_token_address;
use spl_associated_token_account::instruction::create_associated_token_account;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TransactionConfig {
    pub keypair: Arc<Keypair>,
    pub compute_unit_limit: u32,
    pub compute_unit_price: u64,
    pub tip: u64,
    pub buy_amount: u64,
    pub min_amount_out: u64,
}

#[derive(Clone, Debug)]
pub struct PoolVaultInfo {
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
    pub wsol_is_token_a: bool,
}

impl From<PingThingsArgs> for TransactionConfig {
    fn from(args: PingThingsArgs) -> Self {
        let keypair = Keypair::from_base58_string(args.private_key.as_str());

        let tip: u64 = (args.tip * LAMPORTS_PER_SOL as f64) as u64;
        let buy_amount: u64 = (args.buy_amount * LAMPORTS_PER_SOL as f64) as u64;
        let min_amount_out: u64 = (args.min_amount_out * 1_000_000 as f64) as u64;

        TransactionConfig {
            keypair: Arc::new(keypair),
            compute_unit_limit: args.compute_unit_limit,
            compute_unit_price: args.compute_unit_price,
            tip,
            buy_amount,
            min_amount_out,
        }
    }
}

pub fn build_transaction_with_config(
    tx_config: &TransactionConfig,
    rpc_type: &RpcType,
    recent_blockhash: Hash,
    target_token: Pubkey,
    pool_vault_info: PoolVaultInfo,
) -> VersionedTransaction {
    let mut instructions = Vec::new();

    if tx_config.compute_unit_limit > 0 {
        let compute_unit_limit = ComputeBudgetInstruction::set_compute_unit_limit(tx_config.compute_unit_limit);
        instructions.push(compute_unit_limit);
    }

    if tx_config.compute_unit_price > 0 {
        let compute_unit_price = ComputeBudgetInstruction::set_compute_unit_price(tx_config.compute_unit_price);
        instructions.push(compute_unit_price);
    }

    if tx_config.tip > 0 {
        let tip_instruction: Option<Instruction> = match rpc_type {
            RpcType::Jito => Some(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &Pubkey::from_str(JITO_TIP_ADDR).unwrap(),
                tx_config.tip,
            )),
            _ => None,
        };

        if let Some(tip) = tip_instruction {
            instructions.push(tip);
        }
    }

    let meteora_program = Pubkey::from_str(METEORA_PROGRAM_ADDR).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap();

    let owner = tx_config.keypair.pubkey();

    let user_wsol_ata = get_associated_token_address(&owner, &wsol_mint);
    let user_target_ata = get_associated_token_address(&owner, &target_token);

    let create_wsol_ata = create_associated_token_account(&owner, &owner, &wsol_mint, &token_program);
    instructions.push(create_wsol_ata);

    let create_target_ata = create_associated_token_account(&owner, &owner, &target_token, &token_program);
    instructions.push(create_target_ata);

    let wrap_sol_instruction = system_instruction::transfer(&owner, &user_wsol_ata, tx_config.buy_amount);
    instructions.push(wrap_sol_instruction);

    let sync_native_instruction = create_sync_native_instruction(&token_program, &user_wsol_ata);
    instructions.push(sync_native_instruction);

    let swap_data = build_meteora_swap_data(tx_config.buy_amount, tx_config.min_amount_out);

    let (user_source_token, user_destination_token, protocol_fee_account) = if pool_vault_info.wsol_is_token_a {
        (user_wsol_ata, user_target_ata, pool_vault_info.protocol_token_a_fee)
    } else {
        (user_wsol_ata, user_target_ata, pool_vault_info.protocol_token_b_fee)
    };

    let vault_program = Pubkey::from_str("24Uqj9JCLxUeoC3hGfh5W3s9FM9uCHDS2SG3LYwBpyTi").unwrap();

    let meteora_swap_accounts = vec![
        AccountMeta::new(pool_vault_info.pool_address, false),
        AccountMeta::new(user_source_token, false),
        AccountMeta::new(user_destination_token, false),
        AccountMeta::new(pool_vault_info.a_vault, false),
        AccountMeta::new(pool_vault_info.b_vault, false),
        AccountMeta::new(pool_vault_info.a_token_vault, false),
        AccountMeta::new(pool_vault_info.b_token_vault, false),
        AccountMeta::new(pool_vault_info.a_vault_lp_mint, false),
        AccountMeta::new(pool_vault_info.b_vault_lp_mint, false),
        AccountMeta::new(pool_vault_info.a_vault_lp, false),
        AccountMeta::new(pool_vault_info.b_vault_lp, false),
        AccountMeta::new(protocol_fee_account, false),
        AccountMeta::new_readonly(owner, true),
        AccountMeta::new_readonly(vault_program, false),
        AccountMeta::new_readonly(token_program, false),
    ];

    let meteora_swap = Instruction {
        program_id: meteora_program,
        accounts: meteora_swap_accounts,
        data: swap_data,
    };

    instructions.push(meteora_swap);

    let message_v0 = Message::try_compile(&owner, instructions.as_slice(), &[], recent_blockhash).unwrap();

    let versioned_message = VersionedMessage::V0(message_v0);

    VersionedTransaction::try_new(versioned_message, &[&tx_config.keypair]).unwrap()
}

fn build_meteora_swap_data(in_amount: u64, minimum_out_amount: u64) -> Vec<u8> {
    let mut data = vec![248, 198, 158, 145, 225, 117, 135, 200];

    data.extend_from_slice(&in_amount.to_le_bytes());
    data.extend_from_slice(&minimum_out_amount.to_le_bytes());

    data
}

fn create_sync_native_instruction(token_program: &Pubkey, wsol_account: &Pubkey) -> Instruction {
    let data = vec![17];

    let accounts = vec![AccountMeta::new(*wsol_account, false)];

    Instruction {
        program_id: *token_program,
        accounts,
        data,
    }
}
