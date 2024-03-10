use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

use crate::{accounts_zerocopy::KeyedAccountReader, error::MangoError};

use super::{
    pyth_mainnet_sol_oracle, pyth_mainnet_usdc_oracle, sol_mint_mainnet, usdc_mint_mainnet,
};

pub mod orca_mainnet_whirlpool {
    use solana_program::declare_id;
    declare_id!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
}

pub const ORCA_WHIRLPOOL_LEN: usize = 653;
pub const ORCA_WHIRLPOOL_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];

pub struct WhirlpoolState {
    // Q64.64
    pub sqrt_price: u128,     // 16
    pub token_mint_a: Pubkey, // 32
    pub token_mint_b: Pubkey, // 32
}

impl WhirlpoolState {
    pub fn is_inverted(&self) -> bool {
        self.token_mint_a == usdc_mint_mainnet::ID
            || (self.token_mint_a == sol_mint_mainnet::ID
                && self.token_mint_b != usdc_mint_mainnet::ID)
    }

    pub fn get_quote_oracle(&self) -> Result<Pubkey> {
        let mint = if self.is_inverted() {
            self.token_mint_a
        } else {
            self.token_mint_b
        };

        if mint == usdc_mint_mainnet::ID {
            return Ok(pyth_mainnet_usdc_oracle::ID);
        } else if mint == sol_mint_mainnet::ID {
            return Ok(pyth_mainnet_sol_oracle::ID);
        } else {
            return Err(MangoError::MissingFeedForCLMMOracle.into());
        }
    }
}

pub fn load_whirlpool_state(acc_info: &impl KeyedAccountReader) -> Result<WhirlpoolState> {
    let data = &acc_info.data();
    require!(
        data[0..8] == ORCA_WHIRLPOOL_DISCRIMINATOR[..],
        MangoError::InvalidCLMMOracle
    );
    require!(
        data.len() == ORCA_WHIRLPOOL_LEN,
        MangoError::InvalidCLMMOracle
    );
    require!(
        acc_info.owner() == &orca_mainnet_whirlpool::ID,
        MangoError::InvalidCLMMOracle
    );

    let price_bytes: &[u8; 16] = &data[65..81].try_into().unwrap();
    let sqrt_price = u128::from_le_bytes(*price_bytes);
    let a: &[u8; 32] = &(&data[101..133]).try_into().unwrap();
    let b: &[u8; 32] = &(&data[181..213]).try_into().unwrap();
    let mint_a = Pubkey::from(*a);
    let mint_b = Pubkey::from(*b);

    Ok(WhirlpoolState {
        sqrt_price,
        token_mint_a: mint_a,
        token_mint_b: mint_b,
    })
}
