#![allow(dead_code)]

use bytemuck::{bytes_of, Contiguous};
use fixed::types::I80F48;
use mango_v4::state::{PerpMarket, PerpPosition};
use solana_program::instruction::InstructionError;
use solana_program::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::transaction::TransactionError;
use solana_sdk::transport::TransportError;
use std::ops::Deref;

pub fn gen_signer_seeds<'a>(nonce: &'a u64, acc_pk: &'a Pubkey) -> [&'a [u8]; 2] {
    [acc_pk.as_ref(), bytes_of(nonce)]
}

pub fn gen_signer_key(
    nonce: u64,
    acc_pk: &Pubkey,
    program_id: &Pubkey,
) -> Result<Pubkey, ProgramError> {
    let seeds = gen_signer_seeds(&nonce, acc_pk);
    Ok(Pubkey::create_program_address(&seeds, program_id)?)
}

pub fn create_signer_key_and_nonce(program_id: &Pubkey, acc_pk: &Pubkey) -> (Pubkey, u64) {
    for i in 0..=u64::MAX_VALUE {
        if let Ok(pk) = gen_signer_key(i, acc_pk, program_id) {
            return (pk, i);
        }
    }
    panic!("Could not generate signer key");
}

pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_base58_string(&keypair.to_base58_string())
}

// Add clone() to Keypair, totally safe in tests
pub trait ClonableKeypair {
    fn clone(&self) -> Self;
}
impl ClonableKeypair for Keypair {
    fn clone(&self) -> Self {
        clone_keypair(self)
    }
}

// Make a clonable and defaultable Keypair newtype
pub struct TestKeypair(pub Keypair);
impl Clone for TestKeypair {
    fn clone(&self) -> Self {
        TestKeypair(self.0.clone())
    }
}
impl Default for TestKeypair {
    fn default() -> Self {
        TestKeypair(Keypair::from_bytes(&[0u8; 64]).unwrap())
    }
}
impl AsRef<Keypair> for TestKeypair {
    fn as_ref(&self) -> &Keypair {
        &self.0
    }
}
impl Deref for TestKeypair {
    type Target = Keypair;
    fn deref(&self) -> &Keypair {
        &self.0
    }
}
impl From<&Keypair> for TestKeypair {
    fn from(k: &Keypair) -> Self {
        Self(k.clone())
    }
}
impl From<Keypair> for TestKeypair {
    fn from(k: Keypair) -> Self {
        Self(k)
    }
}

pub fn get_pnl_native(
    perp_position: &PerpPosition,
    perp_market: &PerpMarket,
    oracle_price: I80F48,
) -> I80F48 {
    let contract_size = perp_market.base_lot_size;
    let new_quote_pos =
        I80F48::from_num(-perp_position.base_position_lots * contract_size) * oracle_price;
    perp_position.quote_position_native - new_quote_pos
}

pub fn assert_mango_error<T>(
    result: &Result<T, TransportError>,
    expected_error: u32,
    comment: String,
) {
    match result {
        Ok(_) => assert!(false, "No error returned"),
        Err(TransportError::TransactionError(tx_err)) => match tx_err {
            TransactionError::InstructionError(_, err) => match err {
                InstructionError::Custom(err_num) => {
                    assert_eq!(*err_num, expected_error, "{}", comment);
                }
                _ => assert!(false, "Not a mango error"),
            },
            _ => assert!(false, "Not a mango error"),
        },
        _ => assert!(false, "Not a mango error"),
    }
}
