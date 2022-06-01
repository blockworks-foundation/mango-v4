use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;

use crate::checked_math as cm;
use crate::error::MangoError;
use crate::util::LoadZeroCopy;

pub const QUOTE_DECIMALS: u32 = 6;

#[derive(PartialEq)]
pub enum OracleType {
    Stub,
    Pyth,
}

#[account(zero_copy)]
pub struct StubOracle {
    pub group: Pubkey,
    pub mint: Pubkey,
    pub price: I80F48,
    pub last_updated: i64,
    pub reserved: [u8; 8],
}
const_assert_eq!(size_of::<StubOracle>(), 32 + 32 + 16 + 8 + 8);
const_assert_eq!(size_of::<StubOracle>() % 8, 0);

pub fn determine_oracle_type(data: &[u8]) -> Result<OracleType> {
    if u32::from_le_bytes(data[0..4].try_into().unwrap()) == pyth_sdk_solana::state::MAGIC {
        return Ok(OracleType::Pyth);
    } else if data[0..8] == StubOracle::discriminator() {
        return Ok(OracleType::Stub);
    }

    Err(MangoError::UnknownOracleType.into())
}

pub fn oracle_price(acc_info: &AccountInfo, base_token_decimals: u8) -> Result<I80F48> {
    let data = &acc_info.try_borrow_data()?;
    let oracle_type = determine_oracle_type(data)?;

    Ok(match oracle_type {
        OracleType::Stub => acc_info.load::<StubOracle>()?.price,
        OracleType::Pyth => {
            let price_struct = pyth_sdk_solana::load_price(data).unwrap();
            let price = I80F48::from_num(price_struct.price);
            let decimals = (price_struct.expo as u32)
                .checked_add(QUOTE_DECIMALS)
                .unwrap()
                .checked_sub(base_token_decimals as u32)
                .unwrap();
            let decimal_adj = I80F48::from_num(10_u32.pow(decimals));
            cm!(price * decimal_adj)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_test::{find_file, read_file};
    use std::path::PathBuf;

    #[test]
    pub fn test_determine_oracle_type_from_pyth_price_ai() -> Result<()> {
        // add ability to find fixtures
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");

        // load fixture
        // J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix.bin is SOL_PYTH_PRICE
        let filename = "resources/test/J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix.bin";
        let pyth_price_data = read_file(find_file(filename).unwrap());

        assert!(determine_oracle_type(&pyth_price_data).unwrap() == OracleType::Pyth);
        let price = pyth_sdk_solana::load_price(pyth_price_data.as_slice()).unwrap();
        assert_eq!(price.price, 32112500000);

        Ok(())
    }
}
