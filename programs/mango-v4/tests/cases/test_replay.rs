use super::*;

use solana_address_lookup_table_program::state::AddressLookupTable;
use solana_program::program_pack::Pack;
use solana_sdk::account::{Account, ReadableAccount};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::v0::LoadedAddresses;
use solana_sdk::message::SanitizedMessage;
use solana_sdk::message::SanitizedVersionedMessage;
use solana_sdk::message::SimpleAddressLoader;
use solana_sdk::transaction::VersionedTransaction;

use anyhow::Context;
use mango_v4::accounts_zerocopy::LoadMutZeroCopy;
use std::str::FromStr;

fn read_json_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<serde_json::Value> {
    let file_contents = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&file_contents)?;
    Ok(json)
}

fn account_from_snapshot(snapshot_path: &str, pk: Pubkey) -> anyhow::Result<Account> {
    let file_path = format!("{}/{}.json", snapshot_path, pk);
    let json = read_json_file(&file_path).with_context(|| format!("reading {file_path}"))?;
    let account = json.get("account").unwrap();
    let data = base64::decode(
        account.get("data").unwrap().as_array().unwrap()[0]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let owner = Pubkey::from_str(account.get("owner").unwrap().as_str().unwrap()).unwrap();
    let mut account = Account::new(u64::MAX, data.len(), &owner);
    account.data = data;
    Ok(account)
}

fn find_tx(block_file: &str, txsig: &str) -> Option<(u64, i64, Vec<u8>)> {
    let txsig = bs58::decode(txsig).into_vec().unwrap();
    let block = read_json_file(block_file).unwrap();
    let slot = block.get("parentSlot").unwrap().as_u64().unwrap();
    let time = block.get("blockTime").unwrap().as_i64().unwrap();
    let txs = block.get("transactions").unwrap().as_array().unwrap();
    for tx_obj in txs {
        let tx_bytes = base64::decode(
            tx_obj.get("transaction").unwrap().as_array().unwrap()[0]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let sig = &tx_bytes[1..65];
        if sig == txsig {
            return Some((slot, time, tx_bytes.to_vec()));
        }
    }
    None
}

#[tokio::test]
async fn test_replay() -> anyhow::Result<()> {
    // Path to a directory generated with cli save-snapshot, containing <pubkey>.json files
    let snapshot_path = &"path/to/directory";
    // Path to the block data, retrieved with `solana block 252979760 --output json`
    let block_file = &"path/to/block";
    // TX signature in the block that should be looked at
    let txsig = &"";
    // 0-based index of instuction in the tx to try replaying
    let ix_index = 3;

    if txsig.is_empty() {
        return Ok(());
    }

    let mut test_builder = TestContextBuilder::new();
    test_builder.test().set_compute_max_units(400_000);
    let context = test_builder.start_default().await;
    let solana = &context.solana.clone();
    let signer = context.users[0].key;

    let known_accounts = [
        "ComputeBudget111111111111111111111111111111",
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
        "4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg",
        "11111111111111111111111111111111",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    ]
    .iter()
    .map(|s| Pubkey::from_str(s).unwrap())
    .collect_vec();

    // Load block, find tx
    let (slot, time, tx_bytes) = find_tx(block_file, txsig).unwrap();
    let tx: VersionedTransaction = bincode::deserialize(&tx_bytes).unwrap();

    // Lookup ALTs so we can decompile
    let loaded_addresses: LoadedAddresses = tx
        .message
        .address_table_lookups()
        .unwrap_or_default()
        .iter()
        .map(|alt_lookup| {
            let alt_account = account_from_snapshot(snapshot_path, alt_lookup.account_key).unwrap();
            let alt = AddressLookupTable::deserialize(&alt_account.data()).unwrap();
            LoadedAddresses {
                readonly: alt_lookup
                    .readonly_indexes
                    .iter()
                    .map(|i| alt.addresses[*i as usize])
                    .collect_vec(),
                writable: alt_lookup
                    .writable_indexes
                    .iter()
                    .map(|i| alt.addresses[*i as usize])
                    .collect_vec(),
            }
        })
        .collect();
    let alt_loader = SimpleAddressLoader::Enabled(loaded_addresses);

    // decompile instructions, looking up alts at the same time
    let sv_message = SanitizedVersionedMessage::try_from(tx.message).unwrap();
    let s_message = SanitizedMessage::try_new(sv_message, alt_loader).unwrap();
    let bix = &s_message.decompile_instructions()[ix_index];
    let ix = Instruction {
        program_id: *bix.program_id,
        accounts: bix
            .accounts
            .iter()
            .map(|m| AccountMeta {
                pubkey: *m.pubkey,
                is_writable: m.is_writable,
                is_signer: m.is_signer,
            })
            .collect(),
        data: bix.data.to_vec(),
    };

    // since we can't retain the original signer/blockhash, replace it
    let mut replaced_signers = vec![];
    let mut replaced_ix = ix.clone();
    for meta in &mut replaced_ix.accounts {
        if meta.is_signer {
            replaced_signers.push(meta.pubkey);
            meta.pubkey = signer.pubkey();
        }
    }

    // Load all accounts, reporting missing ones, add found to context
    let mut missing_accounts = vec![];
    for pubkey in replaced_ix.accounts.iter().map(|m| m.pubkey) {
        if known_accounts.contains(&pubkey) || pubkey == signer.pubkey() {
            continue;
        }

        let mut account = match account_from_snapshot(snapshot_path, pubkey) {
            Ok(a) => a,
            Err(e) => {
                println!("error reading account from snapshot: {pubkey}, error {e:?}");
                missing_accounts.push(pubkey);
                continue;
            }
        };

        // Override where the previous signer was an owner
        if replaced_signers.contains(&account.owner) {
            account.owner = signer.pubkey();
        }

        // Override mango account owners or delegates
        if let Ok(mut ma) = account.load_mut::<MangoAccountFixed>() {
            if replaced_signers.contains(&ma.owner) {
                ma.owner = signer.pubkey();
            }
            if replaced_signers.contains(&ma.delegate) {
                ma.delegate = signer.pubkey();
            }
        }

        // Override token account owners
        if account.owner == spl_token::id() {
            if let Ok(mut ta) = spl_token::state::Account::unpack(&account.data) {
                if replaced_signers.contains(&ta.owner) {
                    ta.owner = signer.pubkey();
                }
                spl_token::state::Account::pack(ta, &mut account.data).unwrap();
            }
        }

        let mut program_test_context = solana.context.borrow_mut();
        program_test_context.set_account(&pubkey, &account.into());
    }
    if !missing_accounts.is_empty() {
        println!("There were account reading errors, maybe fetch them:");
        for a in &missing_accounts {
            println!("solana account {a} --output json -o {snapshot_path}/{a}.json");
        }
        anyhow::bail!("accounts were missing");
    }

    // update slot/time to roughly match
    let mut clock = solana.clock().await;
    clock.slot = slot;
    clock.unix_timestamp = time;
    solana.set_clock(&clock);

    // Send transaction
    solana
        .process_transaction(&[replaced_ix], Some(&[signer]))
        .await
        .unwrap();

    Ok(())
}
