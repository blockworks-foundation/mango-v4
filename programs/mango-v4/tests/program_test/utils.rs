#![allow(dead_code)]

use bytemuck::{bytes_of, Contiguous};
use solana_program::instruction::InstructionError;
use solana_program::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::transaction::TransactionError;
use solana_sdk::transport::TransportError;

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

/// A Keypair-like struct that's Clone and Copy and can be into()ed to a Keypair
///
/// The regular Keypair is neither Clone nor Copy because the key data is sensitive
/// and should not be copied needlessly. That just makes things difficult for tests.
#[derive(Clone, Copy, Debug)]
pub struct TestKeypair([u8; 64]);
impl TestKeypair {
    pub fn new() -> Self {
        Keypair::new().into()
    }

    pub fn to_keypair(&self) -> Keypair {
        Keypair::from_bytes(&self.0).unwrap()
    }

    pub fn pubkey(&self) -> Pubkey {
        solana_sdk::signature::Signer::pubkey(&self.to_keypair())
    }
}
impl Default for TestKeypair {
    fn default() -> Self {
        Self([0; 64])
    }
}
impl<T: std::borrow::Borrow<Keypair>> From<T> for TestKeypair {
    fn from(k: T) -> Self {
        Self(k.borrow().to_bytes())
    }
}
impl Into<Keypair> for &TestKeypair {
    fn into(self) -> Keypair {
        self.to_keypair()
    }
}

pub fn assert_mango_error<T>(
    result: &Result<T, TransportError>,
    expected_error: u32,
    comment: String,
) {
    match result {
        Ok(_) => assert!(false, "No error returned"),
        Err(TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(err_num),
        ))) => {
            assert_eq!(*err_num, expected_error, "{}", comment);
        }
        _ => assert!(false, "Not a mango error"),
    }
}

#[macro_export]
macro_rules! assert_eq_f64 {
    ($value:expr, $expected:expr, $max_error:expr $(,)?) => {
        let value = $value;
        let expected = $expected;
        let ok = (value - expected).abs() < $max_error;
        if !ok {
            println!("comparison failed: value: {value}, expected: {expected}");
        }
        assert!(ok);
    };
}

#[macro_export]
macro_rules! assert_eq_fixed_f64 {
    ($value:expr, $expected:expr, $max_error:expr $(,)?) => {
        assert_eq_f64!($value.to_num::<f64>(), $expected, $max_error);
    };
}
