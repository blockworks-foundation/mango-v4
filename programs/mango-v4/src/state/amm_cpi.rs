use anchor_lang::prelude::*;
use fixed::types::{I80F48, U64F64};
use solana_program::pubkey::Pubkey;

use crate::{accounts_zerocopy::KeyedAccountReader, error::MangoError};

use super::{
    get_pyth_state, pyth_mainnet_sol_oracle, pyth_mainnet_usdc_oracle, sol_mint_mainnet,
    usdc_mint_mainnet, OracleAccountInfos, OracleState, QUOTE_DECIMALS, SOL_DECIMALS,
};

pub mod orca_mainnet_whirlpool {
    use solana_program::declare_id;
    declare_id!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
}

pub mod raydium_mainnet {
    use solana_program::declare_id;
    declare_id!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

pub const ORCA_WHIRLPOOL_LEN: usize = 653;
pub const ORCA_WHIRLPOOL_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];

pub const RAYDIUM_POOL_LEN: usize = 1544;
pub const RAYDIUM_POOL_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];

pub struct CLMMPoolState {
    // Q64.64
    pub sqrt_price: u128,     // 16
    pub token_mint_a: Pubkey, // 32
    pub token_mint_b: Pubkey, // 32
}

impl CLMMPoolState {
    pub fn is_inverted(&self) -> bool {
        self.token_mint_a == usdc_mint_mainnet::ID
            || (self.token_mint_a == sol_mint_mainnet::ID
                && self.token_mint_b != usdc_mint_mainnet::ID)
    }

    pub fn get_clmm_price(&self) -> I80F48 {
        if self.is_inverted() {
            let sqrt_price = U64F64::from_bits(self.sqrt_price).to_num::<f64>();
            let inverted_price = sqrt_price * sqrt_price;
            I80F48::from_num(1.0f64 / inverted_price)
        } else {
            let sqrt_price = U64F64::from_bits(self.sqrt_price);
            I80F48::from_num(sqrt_price * sqrt_price)
        }
    }

    pub fn quote_state_unchecked<T: KeyedAccountReader>(
        &self,
        acc_infos: &OracleAccountInfos<T>,
    ) -> Result<OracleState> {
        if self.is_inverted() {
            self.quote_state_inner(acc_infos, &self.token_mint_a)
        } else {
            self.quote_state_inner(acc_infos, &self.token_mint_b)
        }
    }

    fn quote_state_inner<T: KeyedAccountReader>(
        &self,
        acc_infos: &OracleAccountInfos<T>,
        quote_mint: &Pubkey,
    ) -> Result<OracleState> {
        if quote_mint == &usdc_mint_mainnet::ID {
            let usd_feed = acc_infos
                .usdc_opt
                .ok_or_else(|| error!(MangoError::MissingFeedForCLMMOracle))?;
            let usd_state = get_pyth_state(usd_feed, QUOTE_DECIMALS as u8)?;
            return Ok(usd_state);
        } else if quote_mint == &sol_mint_mainnet::ID {
            let sol_feed = acc_infos
                .sol_opt
                .ok_or_else(|| error!(MangoError::MissingFeedForCLMMOracle))?;
            let sol_state = get_pyth_state(sol_feed, SOL_DECIMALS as u8)?;
            return Ok(sol_state);
        } else {
            return Err(MangoError::MissingFeedForCLMMOracle.into());
        }
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

    pub fn has_quote_token(&self) -> bool {
        let has_usdc_token = self.token_mint_a == usdc_mint_mainnet::ID
            || self.token_mint_b == usdc_mint_mainnet::ID;
        let has_sol_token =
            self.token_mint_a == sol_mint_mainnet::ID || self.token_mint_b == sol_mint_mainnet::ID;

        has_usdc_token || has_sol_token
    }
}

pub fn load_orca_pool_state(acc_info: &impl KeyedAccountReader) -> Result<CLMMPoolState> {
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

    Ok(CLMMPoolState {
        sqrt_price,
        token_mint_a: mint_a,
        token_mint_b: mint_b,
    })
}

pub fn load_raydium_pool_state(acc_info: &impl KeyedAccountReader) -> Result<CLMMPoolState> {
    let data = &acc_info.data();
    require!(
        data[0..8] == RAYDIUM_POOL_DISCRIMINATOR[..],
        MangoError::InvalidCLMMOracle
    );
    require!(
        data.len() == RAYDIUM_POOL_LEN,
        MangoError::InvalidCLMMOracle
    );
    require!(
        acc_info.owner() == &raydium_mainnet::ID,
        MangoError::InvalidCLMMOracle
    );

    let price_bytes: &[u8; 16] = &data[253..269].try_into().unwrap();
    let sqrt_price = u128::from_le_bytes(*price_bytes);
    let a: &[u8; 32] = &(&data[73..105]).try_into().unwrap();
    let b: &[u8; 32] = &(&data[105..137]).try_into().unwrap();
    let mint_a = Pubkey::from(*a);
    let mint_b = Pubkey::from(*b);

    Ok(CLMMPoolState {
        sqrt_price,
        token_mint_a: mint_a,
        token_mint_b: mint_b,
    })
}
