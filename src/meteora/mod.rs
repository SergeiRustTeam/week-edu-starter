use crate::{
    bench::Bench,
    config::PingThingsArgs,
    core::extract_instructions,
    tx_senders::constants::{METEORA_ID, WSOL_MINT},
};
use solana_sdk::{hash::Hash, pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, TransactionStatusMeta};
use std::{f32::consts, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, log::info};

use sha2::{Digest, Sha256};

/// 8-byte Anchor discriminator for
/// `initializePermissionlessConstantProductPoolWithConfig2`
//pub const INIT_POOL_DISC: [u8; 8] = [113, 24, 119, 179, 117, 115, 18, 54];
pub const IX_DISC_SIZE: usize = 8;

pub fn anchor_discriminator(ix_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", ix_name));
    let hash = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

/*
"accounts":[
    {"name":"pool","isMut":true,"isSigner":false,"docs":["Pool account (PDA address)"]},
    {"name":"config","isMut":false,"isSigner":false},
    {"name":"lpMint","isMut":true,"isSigner":false,"docs":["LP token mint of the pool"]},
    {"name":"tokenAMint","isMut":false,"isSigner":false,"docs":["Token A mint of the pool. Eg: USDT"]},
    {"name":"tokenBMint","isMut":false,"isSigner":false,"docs":["Token B mint of the pool. Eg: USDC"]},
    {"name":"aVault","isMut":true,"isSigner":false,"docs":["Vault account for token A. Token A of the pool will be deposit / withdraw from this vault account."]},
    {"name":"bVault","isMut":true,"isSigner":false,"docs":["Vault account for token B. Token B of the pool will be deposit / withdraw from this vault account."]},
    {"name":"aTokenVault","isMut":true,"isSigner":false,"docs":["Token vault account of vault A"]},
    {"name":"bTokenVault","isMut":true,"isSigner":false,"docs":["Token vault account of vault B"]},
    {"name":"aVaultLpMint","isMut":true,"isSigner":false,"docs":["LP token mint of vault A"]},
    {"name":"bVaultLpMint","isMut":true,"isSigner":false,"docs":["LP token mint of vault B"]},
    {"name":"aVaultLp","isMut":true,"isSigner":false,"docs":["LP token account of vault A. Used to receive/burn the vault LP upon deposit/withdraw from the vault."]},
    {"name":"bVaultLp","isMut":true,"isSigner":false,"docs":["LP token account of vault B. Used to receive/burn vault LP upon deposit/withdraw from the vault."]},
    {"name":"payerTokenA","isMut":true,"isSigner":false,"docs":["Payer token account for pool token A mint. Used to bootstrap the pool with initial liquidity."]},
    {"name":"payerTokenB","isMut":true,"isSigner":false,"docs":["Admin token account for pool token B mint. Used to bootstrap the pool with initial liquidity."]},
    {"name":"payerPoolLp","isMut":true,"isSigner":false},
    {"name":"protocolTokenAFee","isMut":true,"isSigner":false,"docs":["Protocol fee token account for token A. Used to receive trading fee."]},
    {"name":"protocolTokenBFee","isMut":true,"isSigner":false,"docs":["Protocol fee token account for token B. Used to receive trading fee."]},
    {"name":"payer","isMut":true,"isSigner":true,"docs":["Admin account. This account will be the admin of the pool, and the payer for PDA during initialize pool."]},
    {"name":"rent","isMut":false,"isSigner":false,"docs":["Rent account."]},
    {"name":"mintMetadata","isMut":true,"isSigner":false},
    {"name":"metadataProgram","isMut":false,"isSigner":false},
    {"name":"vaultProgram","isMut":false,"isSigner":false,"docs":["Vault program. The pool will deposit/withdraw liquidity from the vault."]},
    {"name":"tokenProgram","isMut":false,"isSigner":false,"docs":["Token program."]},
    {"name":"associatedTokenProgram","isMut":false,"isSigner":false,"docs":["Associated token program."]},
    {"name":"systemProgram","isMut":false,"isSigner":false,"docs":["System program."]}
],
*/

#[derive(Clone, Debug)]
pub struct InitializePermissionlessConstantProductPoolWithConfig2Accounts {
    pub pool: Pubkey,                     // 0: Pool account (PDA)
    pub config: Pubkey,                   // 1
    pub lp_mint: Pubkey,                  // 2: LP token mint of the pool
    pub token_a_mint: Pubkey,             // 3: Token A mint
    pub token_b_mint: Pubkey,             // 4: Token B mint
    pub a_vault: Pubkey,                  // 5: Vault account for token A
    pub b_vault: Pubkey,                  // 6: Vault account for token B
    pub a_token_vault: Pubkey,            // 7: Token vault account of vault A
    pub b_token_vault: Pubkey,            // 8: Token vault account of vault B
    pub a_vault_lp_mint: Pubkey,          // 9: LP token mint of vault A
    pub b_vault_lp_mint: Pubkey,          // 10: LP token mint of vault B
    pub a_vault_lp: Pubkey,               // 11: LP token account of vault A
    pub b_vault_lp: Pubkey,               // 12: LP token account of vault B
    pub payer_token_a: Pubkey,            // 13: Payer token account for token A
    pub payer_token_b: Pubkey,            // 14: Payer token account for token B
    pub payer_pool_lp: Pubkey,            // 15
    pub protocol_token_a_fee: Pubkey,     // 16: Protocol fee token account for token A
    pub protocol_token_b_fee: Pubkey,     // 17: Protocol fee token account for token B
    pub payer: Pubkey,                    // 18: Admin account / payer
    pub rent: Pubkey,                     // 19: Rent sysvar
    pub mint_metadata: Pubkey,            // 20
    pub metadata_program: Pubkey,         // 21
    pub vault_program: Pubkey,            // 22: Vault program
    pub token_program: Pubkey,            // 23: Token program
    pub associated_token_program: Pubkey, // 24: Associated token program
    pub system_program: Pubkey,           // 25: System program
}
pub const INITIALIZE_PERMISSIONLESS_CONSTANT_PRODUCT_POOL_WITH_CONFIG2_NUM_ACCOUNTS: usize = 26;

///
/// Some of the addresses are predefined consts.
/// But for now it is easier to take them from the
/// captured pool creation instruction an pass though
/// the calls.
///
#[derive(Clone, Debug)]
pub struct SwapAccountsFromPoolCreationInstruction {
    pub pool: Pubkey, // 0
    // 1 token_a_mint for userSourceToken - user provided
    // 2 token_b_mint for userDestinationToken - user provided
    pub a_vault: Pubkey,         // 3
    pub b_vault: Pubkey,         // 4
    pub a_token_vault: Pubkey,   // 5
    pub b_token_vault: Pubkey,   // 6
    pub a_vault_lp_mint: Pubkey, // 7
    pub b_vault_lp_mint: Pubkey, // 8
    pub a_vault_lp: Pubkey,      // 9
    pub b_vault_lp: Pubkey,      // 10
    // 11 protocol_token_fee - user provided
    pub vault_program: Pubkey, // 13
    pub token_program: Pubkey, // 14
}

#[derive(Clone, Debug)]
pub struct SwapAccounts {
    pub pool: Pubkey,                   // 0: Pool account (PDA)
    pub user_source_token: Pubkey,      // 1: User token account (input side)
    pub user_destination_token: Pubkey, // 2: User token account (output side)
    pub a_vault: Pubkey,                // 3: Vault account for token A
    pub b_vault: Pubkey,                // 4: Vault account for token B
    pub a_token_vault: Pubkey,          // 5: Token vault account of vault A
    pub b_token_vault: Pubkey,          // 6: Token vault account of vault B
    pub a_vault_lp_mint: Pubkey,        // 7: LP token mint of vault A
    pub b_vault_lp_mint: Pubkey,        // 8: LP token mint of vault B
    pub a_vault_lp: Pubkey,             // 9: LP token account of vault A
    pub b_vault_lp: Pubkey,             // 10: LP token account of vault B
    pub protocol_token_fee: Pubkey,     // 11: Protocol fee token account
    pub user: Pubkey,                   // 12: User account (signer)
    pub vault_program: Pubkey,          // 13: Vault program
    pub token_program: Pubkey,          // 14: Token program
}
pub const SWAP_ACCOUNTS_NUM_ACCOUNTS: usize = 15;

impl From<InitializePermissionlessConstantProductPoolWithConfig2Accounts> for SwapAccountsFromPoolCreationInstruction {
    fn from(init: InitializePermissionlessConstantProductPoolWithConfig2Accounts) -> Self {
        Self {
            pool: init.pool,
            a_vault: init.a_vault,
            b_vault: init.b_vault,
            a_token_vault: init.a_token_vault,
            b_token_vault: init.b_token_vault,
            a_vault_lp_mint: init.a_vault_lp_mint,
            b_vault_lp_mint: init.b_vault_lp_mint,
            a_vault_lp: init.a_vault_lp,
            b_vault_lp: init.b_vault_lp,
            vault_program: init.vault_program,
            token_program: init.token_program,
        }
    }
}

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::UiTransactionEncoding;

use base64::Engine;
use base64::engine::general_purpose;
use solana_sdk::transaction::Transaction;

use solana_transaction_status::UiTransactionStatusMeta;

use anyhow::{Context, Result, bail};
use base64::engine::{Engine as _, general_purpose::STANDARD as B64};

use solana_transaction_status::EncodedTransaction;

// TODO: use it from a unit test.
pub fn fetch_encoded_transaction(sig: &str, rpc_url: &str) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
    let client = RpcClient::new(rpc_url.to_owned());
    let signature = sig.parse().context("invalid signature")?;

    let tx_resp = client
        .get_transaction_with_config(&signature, RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            max_supported_transaction_version: Some(0),
            ..Default::default()
        })
        .context("transaction not found")?;

    Ok(tx_resp)
}

pub struct MeteoraController {
    config: PingThingsArgs,
    bench: Bench,

    // TODO: make use.
    /// Debounce map: (pool_state, slot) —> already processed?
    seen_pools: Arc<RwLock<std::collections::HashSet<(Pubkey, u64)>>>,
}

impl MeteoraController {
    pub fn new(config: PingThingsArgs, bench: Bench) -> Self {
        Self {
            config,
            bench,
            seen_pools: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Called for every transaction Yellowstone pushes to us.
    pub async fn transaction_handler(
        &mut self,
        signature: Signature,
        tx: VersionedTransaction,
        meta: TransactionStatusMeta,
        _is_vote: bool,
        _slot: u64,
    ) -> anyhow::Result<()> {
        // Skip any transaction that ultimately failed (incl. inner errors)
        if meta.status.is_err() {
            return Ok(());
        }

        // It easy to convert this way.
        let meta: UiTransactionStatusMeta = meta.into();

        // Uncomment to override the transaction for test.
        // TODO: extract to a separate unit test.
        /*/
        let encoded_tx = fetch_encoded_transaction(
            "5QWwTAMs98vsPdYbeKbZvKfJQEbaxvB4XDP1EuNaDMXGyJ2Yu8pxnq21a9xmHuGgraYx8pted1qPA6jQQc2DX4ZH",
            "https://api.mainnet-beta.solana.com",
        )?;
        let tx = encoded_tx.transaction.transaction.decode().unwrap();
        let meta = encoded_tx.transaction.meta.expect("Cannot decode meta");
        */

        debug!("Transaction captured tx={:?}", tx);

        // Flatten top-level + inner instructions
        for ix in extract_instructions(meta, tx.clone())? {

            // Program must be Meteora.
            if ix.program_id != *METEORA_ID {
                continue;
            }

            // TODO: move to constants.
            const CREATE_POOL_DISCRIMINATOR: [u8; 8] = [48, 149, 220, 130, 61, 11, 9, 178];

            // Data must start with the v2 discriminator.
            if ix.data.len() < IX_DISC_SIZE || &ix.data[..IX_DISC_SIZE] != CREATE_POOL_DISCRIMINATOR {
                continue;
            }
            // TODO: debug!()
            info!("Pool creation ix={:?}", ix);            

            if ix.accounts.len() < 26 {
                continue;
            }

            let pool_creation_accounts = InitializePermissionlessConstantProductPoolWithConfig2Accounts {
                pool: ix.accounts[0].pubkey,
                config: ix.accounts[1].pubkey,
                lp_mint: ix.accounts[2].pubkey,
                token_a_mint: ix.accounts[3].pubkey,
                token_b_mint: ix.accounts[4].pubkey,
                a_vault: ix.accounts[5].pubkey,
                b_vault: ix.accounts[6].pubkey,
                a_token_vault: ix.accounts[7].pubkey,
                b_token_vault: ix.accounts[8].pubkey,
                a_vault_lp_mint: ix.accounts[9].pubkey,
                b_vault_lp_mint: ix.accounts[10].pubkey,
                a_vault_lp: ix.accounts[11].pubkey,
                b_vault_lp: ix.accounts[12].pubkey,
                payer_token_a: ix.accounts[13].pubkey,
                payer_token_b: ix.accounts[14].pubkey,
                payer_pool_lp: ix.accounts[15].pubkey,
                protocol_token_a_fee: ix.accounts[16].pubkey,
                protocol_token_b_fee: ix.accounts[17].pubkey,
                payer: ix.accounts[18].pubkey,
                rent: ix.accounts[19].pubkey,
                mint_metadata: ix.accounts[20].pubkey,
                metadata_program: ix.accounts[21].pubkey,
                vault_program: ix.accounts[22].pubkey,
                token_program: ix.accounts[23].pubkey,
                associated_token_program: ix.accounts[24].pubkey,
                system_program: ix.accounts[25].pubkey,
            };

            // TODO: debug!()
            info!("pool_creation_accounts: {:#?}", pool_creation_accounts);

            let token_a_mint = pool_creation_accounts.token_a_mint;
            let token_b_mint = pool_creation_accounts.token_b_mint;
            let protocol_token_a_fee = pool_creation_accounts.protocol_token_a_fee;
            let protocol_token_b_fee = pool_creation_accounts.protocol_token_b_fee;

            // TODO: not all of the accounts need to be taken from the pool creation transaction
            // as they a predefined consts. Though, it is easier to pass them through for now.
            let swap_accounts: SwapAccountsFromPoolCreationInstruction = pool_creation_accounts.clone().into();

            // We only want pools that involve WSOL
            if token_a_mint != *WSOL_MINT && token_b_mint != *WSOL_MINT {
                continue;
            }

            // Debounce so a re-broadcast in the same slot won’t fire twice.
            /*{
                let mut seen = self.seen_pools.write().await;
                if !seen.insert((pool_state, slot)) {
                    continue; // already handled
                }
            }*/

            info!(
                "New WSOL pool detected: {:?}, token_a_mint={}, token_b_mint={}",
                signature, token_a_mint, token_b_mint
            );

            //  Build & send swap txn.
            let recent_blockhash: Hash = *tx.message.recent_blockhash();

            // Which side are we paying with (WSOL)?
            let (token_source_mint, token_dest_mint, protocol_token_fee) = if token_a_mint == *WSOL_MINT {
                (token_a_mint, token_b_mint, protocol_token_a_fee)
            } else {
                (token_b_mint, token_a_mint, protocol_token_b_fee)
            };

            self.bench
                .clone()
                .send_swap_tx(
                    recent_blockhash,
                    token_source_mint,
                    token_dest_mint,
                    protocol_token_fee,
                    swap_accounts,
                )
                .await?;
        }

        Ok(())
    }
}
