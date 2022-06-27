use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use fixed::types::I80F48;

use static_assertions::const_assert_eq;
use switchboard_program::FastRoundResultAccountData;
use switchboard_v2::AggregatorAccountData;

use crate::accounts_zerocopy::*;
use crate::checked_math as cm;
use crate::error::MangoError;

pub const QUOTE_DECIMALS: i8 = 6;

const LOOKUP_START: i8 = -12;
const LOOKUP: [I80F48; 25] = [
    I80F48::from_bits((1 << 48) / 10i128.pow(12u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(11u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(10u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(9u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(8u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(7u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(6u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(5u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(4u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(3u32) + 1), // 0.001
    I80F48::from_bits((1 << 48) / 10i128.pow(2u32) + 1), // 0.01
    I80F48::from_bits((1 << 48) / 10i128.pow(1u32) + 1), // 0.1
    I80F48::from_bits((1 << 48) * 10i128.pow(0u32)),     // 1
    I80F48::from_bits((1 << 48) * 10i128.pow(1u32)),     // 10
    I80F48::from_bits((1 << 48) * 10i128.pow(2u32)),     // 100
    I80F48::from_bits((1 << 48) * 10i128.pow(3u32)),     // 1000
    I80F48::from_bits((1 << 48) * 10i128.pow(4u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(5u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(6u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(7u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(8u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(9u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(10u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(11u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(12u32)),
];
const LOOKUP_FN: fn(i8) -> usize = |decimals: i8| (decimals - LOOKUP_START) as usize;

pub mod switchboard_v1_devnet_oracle {
    use solana_program::declare_id;
    declare_id!("7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU");
}
pub mod switchboard_v2_mainnet_oracle {
    use solana_program::declare_id;
    declare_id!("DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM");
}

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct OracleConfig {
    pub conf_filter: I80F48,
}
const_assert_eq!(size_of::<OracleConfig>(), 16);
const_assert_eq!(size_of::<OracleConfig>() % 8, 0);

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

pub fn determine_oracle_type(acc_info: &impl KeyedAccountReader) -> Result<OracleType> {
    let data = acc_info.data();

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
    else if acc_info.owner() == &switchboard_v1_devnet_oracle::ID
        || acc_info.owner() == &switchboard_v2_mainnet_oracle::ID
    {
        return Ok(OracleType::SwitchboardV1);
    }

    Err(MangoError::UnknownOracleType.into())
}

pub fn oracle_price(
    acc_info: &impl KeyedAccountReader,
    oracle_conf_filter: I80F48,
    base_token_decimals: u8,
) -> Result<I80F48> {
    let data = &acc_info.data();
    let oracle_type = determine_oracle_type(acc_info)?;

    Ok(match oracle_type {
        OracleType::Stub => acc_info.load::<StubOracle>()?.price,
        OracleType::Pyth => {
            let price_account = pyth_sdk_solana::load_price(data).unwrap();
            let price = I80F48::from_num(price_account.price);

            // Filter out bad prices
            if I80F48::from_num(price_account.conf) > oracle_conf_filter * price {
                msg!(
                    "Pyth conf interval too high; pubkey {} price: {} price_account.conf: {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    price_account.conf
                );

                // future: in v3, we had pricecache, and in case of luna, when there were no updates, we used last known value from cache
                // we'll have to add a CachedOracle that is based on one of the oracle types, needs a separate keeper and supports
                // maintaining this "last known good value"
                return Err(MangoError::SomeError.into());
            }

            let decimals = (price_account.expo as i8)
                .checked_add(QUOTE_DECIMALS)
                .unwrap()
                .checked_sub(base_token_decimals as i8)
                .unwrap();
            let decimal_adj = LOOKUP[LOOKUP_FN(decimals)];
            cm!(price * decimal_adj)
        }
        OracleType::SwitchboardV2 => {
            let feed = bytemuck::from_bytes::<AggregatorAccountData>(&data[8..]);
            let feed_result = feed.get_result()?;
            let price_decimal: f64 = feed_result.try_into()?;
            let price = I80F48::from_num(price_decimal);

            // Filter out bad prices
            let std_deviation_decimal: f64 =
                feed.latest_confirmed_round.std_deviation.try_into()?;
            if I80F48::from_num(std_deviation_decimal) > oracle_conf_filter * price {
                msg!(
                    "Switchboard v2 std deviation too high; pubkey {} price: {} latest_confirmed_round.std_deviation: {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    std_deviation_decimal
                );
                return Err(MangoError::SomeError.into());
            }

            let decimals = QUOTE_DECIMALS
                .checked_sub(base_token_decimals as i8)
                .unwrap();
            let decimal_adj = LOOKUP[LOOKUP_FN(decimals)];
            cm!(price * decimal_adj)
        }
        OracleType::SwitchboardV1 => {
            let result = FastRoundResultAccountData::deserialize(data).unwrap();
            let price = I80F48::from_num(result.result.result);

            // Filter out bad prices
            let min_response = I80F48::from_num(result.result.min_response);
            let max_response = I80F48::from_num(result.result.max_response);
            if cm!(max_response - min_response) > oracle_conf_filter * price {
                msg!(
                    "Switchboard v1 min-max response gap too wide; pubkey {} price: {} min_response: {} max_response {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    min_response,
                    max_response
                );
                return Err(MangoError::SomeError.into());
            }

            let decimals = QUOTE_DECIMALS
                .checked_sub(base_token_decimals as i8)
                .unwrap();
            let decimal_adj = LOOKUP[LOOKUP_FN(decimals)];
            cm!(price * decimal_adj)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_test::{find_file, read_file};
    use std::{cell::RefCell, path::PathBuf, str::FromStr};

    #[test]
    pub fn test_oracles() -> Result<()> {
        // add ability to find fixtures
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");

        let fixtures = vec![
            (
                "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix",
                OracleType::Pyth,
                Pubkey::default(),
            ),
            (
                "8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o",
                OracleType::SwitchboardV1,
                switchboard_v1_devnet_oracle::ID,
            ),
            (
                "GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR",
                OracleType::SwitchboardV2,
                Pubkey::default(),
            ),
        ];

        for fixture in fixtures {
            let filename = format!("resources/test/{}.bin", fixture.0);
            let mut pyth_price_data = read_file(find_file(&filename).unwrap());
            let data = RefCell::new(&mut pyth_price_data[..]);
            let ai = &AccountInfoRef {
                key: &Pubkey::from_str(fixture.0).unwrap(),
                owner: &fixture.2,
                data: data.borrow(),
            };
            assert!(determine_oracle_type(ai).unwrap() == fixture.1);
        }

        Ok(())
    }

    #[test]
    pub fn lookup_test() {
        for idx in -12..0 {
            assert_eq!(
                LOOKUP[LOOKUP_FN(idx)],
                I80F48::from_str(&format!(
                    "0.{}1",
                    str::repeat("0", (idx.abs() as usize) - 1)
                ))
                .unwrap()
            )
        }

        assert_eq!(LOOKUP[LOOKUP_FN(0)], I80F48::ONE);

        for idx in 1..=12 {
            assert_eq!(
                LOOKUP[LOOKUP_FN(idx)],
                I80F48::from_str(&format!("1{}", str::repeat("0", idx.abs() as usize))).unwrap()
            )
        }
    }
}
