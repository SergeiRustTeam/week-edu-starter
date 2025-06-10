use crate::geyser::GeyserResult;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::message::VersionedMessage;
use solana_sdk::message::v0::{LoadedAddresses, LoadedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::reserved_account_keys::ReservedAccountKeys;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionStatusMeta;

pub fn extract_instructions(
    meta_data: TransactionStatusMeta,
    transaction: VersionedTransaction,
) -> GeyserResult<Vec<solana_sdk::instruction::Instruction>> {
    let message = transaction.message.clone();
    let meta = meta_data.clone();

    let mut instructions = Vec::<solana_sdk::instruction::Instruction>::new();

    match message {
        VersionedMessage::Legacy(legacy) => {
            for compiled_instruction in legacy.instructions.iter() {
                let program_id = *legacy
                    .account_keys
                    .get(compiled_instruction.program_id_index as usize)
                    .unwrap_or(&Pubkey::default());

                let accounts: Vec<_> = compiled_instruction
                    .accounts
                    .iter()
                    .filter_map(|account_index| {
                        let account_pubkey = legacy.account_keys.get(*account_index as usize)?;
                        Some(AccountMeta {
                            pubkey: *account_pubkey,
                            is_writable: legacy.is_maybe_writable(*account_index as usize, None),
                            is_signer: legacy.is_signer(*account_index as usize),
                        })
                    })
                    .collect();

                instructions.push(solana_sdk::instruction::Instruction {
                    program_id,
                    accounts,
                    data: compiled_instruction.data.clone(),
                });
            }
        }
        VersionedMessage::V0(v0) => {
            let loaded_addresses = LoadedAddresses {
                writable: meta.loaded_addresses.writable.to_vec(),
                readonly: meta.loaded_addresses.readonly.to_vec(),
            };

            let loaded_message =
                LoadedMessage::new(v0.clone(), loaded_addresses, &ReservedAccountKeys::empty_key_set());

            for compiled_instruction in v0.instructions.iter() {
                let program_id = *loaded_message
                    .account_keys()
                    .get(compiled_instruction.program_id_index as usize)
                    .unwrap_or(&Pubkey::default());

                let accounts: Vec<AccountMeta> = compiled_instruction
                    .accounts
                    .iter()
                    .filter_map(|account_index| {
                        let account_pubkey = loaded_message.account_keys().get(*account_index as usize);

                        Some(AccountMeta {
                            pubkey: account_pubkey.copied().unwrap_or_default(),
                            is_writable: loaded_message.is_writable(*account_index as usize),
                            is_signer: loaded_message.is_signer(*account_index as usize),
                        })
                    })
                    .collect();

                instructions.push(solana_sdk::instruction::Instruction {
                    program_id,
                    accounts,
                    data: compiled_instruction.data.clone(),
                });
            }
        }
    }

    Ok(instructions)
}
