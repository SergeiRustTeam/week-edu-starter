use crate::config::{PingThingsArgs, RpcType};
use crate::tx_senders::constants::*;

use crate::meteora::*;

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
use tracing::debug;
use std::str::FromStr;
use std::sync::Arc;

use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct TransactionConfig {
    pub keypair: Arc<Keypair>, // payer + signer
    pub compute_unit_limit: u32,
    pub compute_unit_price: u64,
    pub tip: u64,
    pub buy_amount: u64,
    pub min_amount_out: u64,
    pub activation_point: Option<u64>, // not used yet
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
            activation_point: None,
        }
    }
}

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
struct SwapInstructionData {
    buy_amount: u64,
    min_amount_out: u64,
    activation_point: Option<u64>,
}

pub fn build_meteora_swap_transaction_with_config(
    tx_config: &TransactionConfig,
    rpc_type: &RpcType,
    recent_blockhash: Hash,
    token_source_mint: Pubkey, // always WSOL
    token_dest_mint: Pubkey,
    protocol_token_fee: Pubkey,
    accounts: SwapAccountsFromPoolCreationInstruction,
) -> VersionedTransaction {
    assert!(tx_config.buy_amount > 0, "buy_amount must be > 0");

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
        let tip_instruction = match &rpc_type {
            RpcType::Jito => Some(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &JITO_TIP_PUBKEY,
                tx_config.tip,
            )),
            RpcType::NextBlock => Some(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &NEXTBLOCK_TIP_PUBKEY,
                tx_config.tip,
            )),
            RpcType::Bloxroute => Some(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &BLOXROUTE_TIP_PUBKEY,
                tx_config.tip,
            )),
            _ => None,
        };

        if let Some(tip_instruction) = tip_instruction {
            instructions.push(tip_instruction);
        }
    }

    let token_program_pubkey: Pubkey = Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap();

    let owner = tx_config.keypair.pubkey();

    let user_source_token = get_associated_token_address(&owner, &token_source_mint);
    let user_destination_token = get_associated_token_address(&owner, &token_dest_mint);

    let token_account_instruction =
        create_associated_token_account(&owner, &owner, &token_dest_mint, &token_program_pubkey);
    instructions.push(token_account_instruction);

    let swap_accounts = SwapAccounts {
        pool: accounts.pool,
        user_source_token,
        user_destination_token,
        a_vault: accounts.a_vault,
        b_vault: accounts.b_vault,
        a_token_vault: accounts.a_token_vault,
        b_token_vault: accounts.b_token_vault,
        a_vault_lp_mint: accounts.a_vault_lp_mint,
        b_vault_lp_mint: accounts.b_vault_lp_mint,
        a_vault_lp: accounts.a_vault_lp,
        b_vault_lp: accounts.b_vault_lp,
        protocol_token_fee,
        user: owner,
        vault_program: accounts.vault_program,
        token_program: accounts.token_program,
    };

    const METEORA_SWP_IX_DISC: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
    //let swap_discriminator = anchor_discriminator("swap");

    /*let mut data = vec![];
    data.extend_from_slice(&METEORA_SWP_IX_DISC);
    data.extend_from_slice(&tx_config.buy_amount.to_le_bytes()); // inAmount
    data.extend_from_slice(&tx_config.min_amount_out.to_le_bytes()); // minimumOutAmount
    data.push(0x00 as u8); // Option::None for activation_point*/

    let swap_data = SwapInstructionData {
        buy_amount: tx_config.buy_amount,
        min_amount_out: tx_config.min_amount_out,
        activation_point: None,
    };
    let swap_data = borsh::to_vec(&swap_data).expect("failed to serialize swap data");
    let mut data = vec![];
    data.extend_from_slice(&METEORA_SWP_IX_DISC);
    data.extend_from_slice(&swap_data);

    assert!(data.len() == 25 || data.len() == 33, "Unexpected swap data length");

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool, false),                   // 0: pool
        AccountMeta::new(swap_accounts.user_source_token, false),      // 1: userSourceToken
        AccountMeta::new(swap_accounts.user_destination_token, false), // 2: userDestinationToken
        AccountMeta::new(swap_accounts.a_vault, false),                // 3: aVault
        AccountMeta::new(swap_accounts.b_vault, false),                // 4: bVault
        AccountMeta::new(swap_accounts.a_token_vault, false),          // 5: aTokenVault
        AccountMeta::new(swap_accounts.b_token_vault, false),          // 6: bTokenVault
        AccountMeta::new(swap_accounts.a_vault_lp_mint, false),        // 7: aVaultLpMint
        AccountMeta::new(swap_accounts.b_vault_lp_mint, false),        // 8: bVaultLpMint
        AccountMeta::new(swap_accounts.a_vault_lp, false),             // 9: aVaultLp
        AccountMeta::new(swap_accounts.b_vault_lp, false),             // 10: bVaultLp
        AccountMeta::new(swap_accounts.protocol_token_fee, false),     // 11: protocolTokenFee
        AccountMeta::new_readonly(swap_accounts.user, true),           // 12: user
        AccountMeta::new_readonly(swap_accounts.vault_program, false), // 13: vaultProgram
        AccountMeta::new_readonly(swap_accounts.token_program, false), // 14: tokenProgram
    ];

    let swap_instruction = Instruction {
        program_id: Pubkey::from_str(METEORA_PROGRAM_ID).unwrap(),
        accounts,
        data,
    };

    instructions.push(swap_instruction);

    for (i, ix) in instructions.iter().enumerate() {
        debug!("Instruction {}: {:?}", i, ix);
    }

    let message = Message::try_compile(&owner, &instructions, &[], recent_blockhash).expect("compile message failed");
    let versioned_msg = VersionedMessage::V0(message);
    VersionedTransaction::try_new(versioned_msg, &[&*tx_config.keypair]).expect("failed to build versioned transaction")
}
