use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use static_assertions::const_assert_eq;
use switchboard_program::FastRoundResultAccountData;
use switchboard_v2::AggregatorAccountData;

use crate::accounts_zerocopy::*;
use crate::checked_math as cm;
use crate::error::MangoError;

pub const PYTH_CONF_FILTER: I80F48 = I80F48!(0.10); // filter out pyth prices with conf > 10% of price
pub const QUOTE_DECIMALS: i32 = 6;

#[derive(PartialEq)]
pub enum OracleType {
    Pyth,
    Stub,
    SwitchboardV1,
    SwitchboardV2,
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
    // https://github.com/switchboard-xyz/switchboard-v2/blob/main/libraries/rs/src/aggregator.rs#L114
    // note: disc is not public, hence the copy pasta
    else if data[0..8] == [217, 230, 65, 101, 201, 162, 27, 125] {
        return Ok(OracleType::SwitchboardV2);
    }
    // note: this is the only known way of checking this
    else if data.len() == 1000 {
        return Ok(OracleType::SwitchboardV1);
    }

    Err(MangoError::UnknownOracleType.into())
}

pub fn oracle_price(
    acc_info: &impl AccountReader,
    pubkey: &Pubkey,
    base_token_decimals: u8,
) -> Result<I80F48> {
    let data = &acc_info.data();
    let oracle_type = determine_oracle_type(data)?;

    Ok(match oracle_type {
        OracleType::Stub => acc_info.load::<StubOracle>()?.price,
        OracleType::Pyth => {
            let price_account = pyth_sdk_solana::load_price(data).unwrap();
            let price = I80F48::from_num(price_account.price);

            // Filter out bad prices on mainnet
            #[cfg(not(feature = "devnet"))]
            let conf = I80F48::from_num(price_account.conf)
                .checked_div(price)
                .unwrap();

            #[cfg(not(feature = "devnet"))]
            if conf > PYTH_CONF_FILTER {
                msg!(
                    "Pyth conf interval too high; pubkey {} price: {} conf: {}",
                    pubkey,
                    price.to_num::<f64>(),
                    conf.to_num::<f64>()
                );

                // future: in v3, we had pricecache, and in case of luna, when there were no updates, we used last known value from cache, any
                // suggestions now that we dont have a cache?

                return Err(MangoError::SomeError.into());
            }

            let decimals = (price_account.expo as i32)
                .checked_add(QUOTE_DECIMALS)
                .unwrap()
                .checked_sub(base_token_decimals as i32)
                .unwrap();
            let decimal_adj = I80F48::from_num(10_u32.pow(decimals.abs() as u32));
            if decimals < 0 {
                cm!(price / decimal_adj)
            } else {
                cm!(price * decimal_adj)
            }
        }
        OracleType::SwitchboardV2 => {
            let feed_result =
                bytemuck::from_bytes::<AggregatorAccountData>(&data[8..]).get_result()?;
            let decimal: f64 = feed_result.try_into()?;
            let price = I80F48::from_num(decimal);
            let decimals = QUOTE_DECIMALS
                .checked_sub(base_token_decimals as i32)
                .unwrap();
            let decimal_adj = I80F48::from_num(10u64.pow(decimals.abs() as u32));
            if decimals < 0 {
                cm!(price / decimal_adj)
            } else {
                cm!(price * decimal_adj)
            }
        }
        OracleType::SwitchboardV1 => {
            let result = FastRoundResultAccountData::deserialize(data).unwrap();
            let price = I80F48::from_num(result.result.result);
            let decimals = QUOTE_DECIMALS
                .checked_sub(base_token_decimals as i32)
                .unwrap();
            let decimal_adj = I80F48::from_num(10u64.pow(decimals.abs() as u32));
            if decimals < 0 {
                cm!(price / decimal_adj)
            } else {
                cm!(price * decimal_adj)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_test::{find_file, read_file};
    use std::path::PathBuf;

    #[test]
    pub fn test_oracles() -> Result<()> {
        // add ability to find fixtures
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");

        let fixtures = vec![
            (
                "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix",
                OracleType::Pyth,
            ),
            (
                "8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o",
                OracleType::SwitchboardV1,
            ),
            (
                "GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR",
                OracleType::SwitchboardV2,
            ),
        ];

        for fixture in fixtures {
            let filename = format!("resources/test/{}.bin", fixture.0);
            let pyth_price_data = read_file(find_file(&filename).unwrap());
            assert!(determine_oracle_type(&pyth_price_data).unwrap() == fixture.1);
        }

        Ok(())
    }
}
