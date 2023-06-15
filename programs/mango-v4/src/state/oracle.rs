use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use fixed::types::I80F48;

use static_assertions::const_assert_eq;
use switchboard_program::FastRoundResultAccountData;
use switchboard_v2::AggregatorAccountData;

use crate::accounts_zerocopy::*;

use crate::error::*;

const DECIMAL_CONSTANT_ZERO_INDEX: i8 = 12;
const DECIMAL_CONSTANTS: [I80F48; 25] = [
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
    I80F48::from_bits((1 << 48) * 10i128.pow(0u32)),     // 1, index 12
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
pub const fn power_of_ten(decimals: i8) -> I80F48 {
    DECIMAL_CONSTANTS[(decimals + DECIMAL_CONSTANT_ZERO_INDEX) as usize]
}

pub const QUOTE_DECIMALS: i8 = 6;
pub const QUOTE_NATIVE_TO_UI: I80F48 = power_of_ten(-QUOTE_DECIMALS);

pub mod switchboard_v1_devnet_oracle {
    use solana_program::declare_id;
    declare_id!("7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU");
}
pub mod switchboard_v2_mainnet_oracle {
    use solana_program::declare_id;
    declare_id!("DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM");
}

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Debug, bytemuck::Pod)]
pub struct OracleConfig {
    pub conf_filter: I80F48,
    pub max_staleness_slots: i64,
    pub reserved: [u8; 72],
}
const_assert_eq!(size_of::<OracleConfig>(), 16 + 8 + 72);
const_assert_eq!(size_of::<OracleConfig>(), 96);
const_assert_eq!(size_of::<OracleConfig>() % 8, 0);

#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct OracleConfigParams {
    pub conf_filter: f32,
    pub max_staleness_slots: Option<u32>,
}

impl OracleConfigParams {
    pub fn to_oracle_config(&self) -> OracleConfig {
        OracleConfig {
            conf_filter: I80F48::from_num(self.conf_filter),
            max_staleness_slots: self.max_staleness_slots.map(|v| v as i64).unwrap_or(-1),
            reserved: [0; 72],
        }
    }
}

#[derive(PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum OracleType {
    Pyth,
    Stub,
    SwitchboardV1,
    SwitchboardV2,
}

pub struct OracleState {
    pub last_update_slot: u64,
    pub confidence: I80F48,
    pub oracle_type: OracleType,
}

#[account(zero_copy)]
pub struct StubOracle {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,
    // ABI: Clients rely on this being at offset 40
    pub mint: Pubkey,
    pub price: I80F48,
    pub last_updated: i64,
    pub reserved: [u8; 128],
}
const_assert_eq!(size_of::<StubOracle>(), 32 + 32 + 16 + 8 + 128);
const_assert_eq!(size_of::<StubOracle>(), 216);
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

/// Returns the price of one native base token, in native quote tokens
///
/// Example: The for SOL at 40 USDC/SOL it would return 0.04 (the unit is USDC-native/SOL-native)
///
/// This currently assumes that quote decimals is 6, like for USDC.
///
/// Pass `staleness_slot` = None to skip the staleness check
pub fn oracle_price_and_state(
    acc_info: &impl KeyedAccountReader,
    config: &OracleConfig,
    base_decimals: u8,
    staleness_slot: Option<u64>,
) -> Result<(I80F48, OracleState)> {
    let data = &acc_info.data();
    let oracle_type = determine_oracle_type(acc_info)?;
    let staleness_slot = staleness_slot.unwrap_or(0);

    Ok(match oracle_type {
        OracleType::Stub => (
            acc_info.load::<StubOracle>()?.price,
            OracleState {
                last_update_slot: 0,
                confidence: I80F48::ZERO,
                oracle_type: OracleType::Stub,
            },
        ),
        OracleType::Pyth => {
            let price_account = pyth_sdk_solana::state::load_price_account(data).unwrap();
            let price_data = price_account.to_price();
            let price = I80F48::from_num(price_data.price);

            // Don't use price_data.status, because that has its own built-in staleness detection,
            // check PriceAccount::to_price() impl.
            if price_account.agg.status != pyth_sdk_solana::PriceStatus::Trading {
                msg!(
                    "Pyth price status isn't 'Trading': status: {}",
                    price_data.status as u64
                );

                return Err(MangoError::OracleStale.into());
            }

            // Filter out bad prices
            if I80F48::from_num(price_data.conf) > (config.conf_filter * price) {
                msg!(
                    "Pyth conf interval too high; pubkey {} price: {} price_data.conf: {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    price_data.conf
                );

                // future: in v3, we had pricecache, and in case of luna, when there were no updates, we used last known value from cache
                // we'll have to add a CachedOracle that is based on one of the oracle types, needs a separate keeper and supports
                // maintaining this "last known good value"
                return Err(MangoError::OracleConfidence.into());
            }

            // The last_slot is when the price was actually updated
            let last_slot = price_account.last_slot;
            if config.max_staleness_slots >= 0
                && price_account
                    .last_slot
                    .saturating_add(config.max_staleness_slots as u64)
                    < staleness_slot
            {
                msg!(
                    "Pyth price too stale; pubkey {} price: {} last slot: {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    last_slot,
                );

                return Err(MangoError::OracleStale.into());
            }

            let decimals = (price_account.expo as i8) + QUOTE_DECIMALS - (base_decimals as i8);
            let decimal_adj = power_of_ten(decimals);
            (
                price * decimal_adj,
                OracleState {
                    last_update_slot: last_slot,
                    confidence: I80F48::from_num(price_data.conf),
                    oracle_type: OracleType::Pyth,
                },
            )
        }
        OracleType::SwitchboardV2 => {
            fn from_foreign_error(e: impl std::fmt::Display) -> Error {
                error_msg!("{}", e)
            }

            let feed = bytemuck::from_bytes::<AggregatorAccountData>(&data[8..]);
            let feed_result = feed.get_result().map_err(from_foreign_error)?;
            let price_decimal: f64 = feed_result.try_into().map_err(from_foreign_error)?;
            let price = I80F48::from_num(price_decimal);

            // Filter out bad prices
            let std_deviation_decimal: f64 = feed
                .latest_confirmed_round
                .std_deviation
                .try_into()
                .map_err(from_foreign_error)?;
            if I80F48::from_num(std_deviation_decimal) > (config.conf_filter * price) {
                msg!(
                    "Switchboard v2 std deviation too high; pubkey {} price: {} latest_confirmed_round.std_deviation: {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    std_deviation_decimal
                );
                return Err(MangoError::OracleConfidence.into());
            }

            // The round_open_slot is an overestimate of the oracle staleness: Reporters will see
            // the round opening and only then start executing the price tasks.
            let round_open_slot = feed.latest_confirmed_round.round_open_slot;
            if config.max_staleness_slots >= 0
                && round_open_slot.saturating_add(config.max_staleness_slots as u64)
                    < staleness_slot
            {
                msg!(
                    "Switchboard v2 price too stale; pubkey {} price: {} latest_confirmed_round.round_open_slot: {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    round_open_slot,
                );
                return Err(MangoError::OracleConfidence.into());
            }

            let decimals = QUOTE_DECIMALS - (base_decimals as i8);
            let decimal_adj = power_of_ten(decimals);
            (
                price * decimal_adj,
                OracleState {
                    last_update_slot: round_open_slot,
                    confidence: I80F48::from_num(std_deviation_decimal),
                    oracle_type: OracleType::SwitchboardV2,
                },
            )
        }
        OracleType::SwitchboardV1 => {
            let result = FastRoundResultAccountData::deserialize(data).unwrap();
            let price = I80F48::from_num(result.result.result);

            // Filter out bad prices
            let min_response = I80F48::from_num(result.result.min_response);
            let max_response = I80F48::from_num(result.result.max_response);
            if (max_response - min_response) > (config.conf_filter * price) {
                msg!(
                    "Switchboard v1 min-max response gap too wide; pubkey {} price: {} min_response: {} max_response {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    min_response,
                    max_response
                );
                return Err(MangoError::OracleConfidence.into());
            }

            let round_open_slot = result.result.round_open_slot;
            if config.max_staleness_slots >= 0
                && round_open_slot.saturating_add(config.max_staleness_slots as u64)
                    < staleness_slot
            {
                msg!(
                    "Switchboard v1 price too stale; pubkey {} price: {} round_open_slot: {}",
                    acc_info.key(),
                    price.to_num::<f64>(),
                    round_open_slot,
                );
                return Err(MangoError::OracleConfidence.into());
            }

            let decimals = QUOTE_DECIMALS - (base_decimals as i8);
            let decimal_adj = power_of_ten(decimals);
            (
                price * decimal_adj,
                OracleState {
                    last_update_slot: round_open_slot,
                    confidence: max_response - min_response,
                    oracle_type: OracleType::SwitchboardV1,
                },
            )
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
                power_of_ten(idx),
                I80F48::from_str(&format!(
                    "0.{}1",
                    str::repeat("0", (idx.abs() as usize) - 1)
                ))
                .unwrap()
            )
        }

        assert_eq!(power_of_ten(0), I80F48::ONE);

        for idx in 1..=12 {
            assert_eq!(
                power_of_ten(idx),
                I80F48::from_str(&format!("1{}", str::repeat("0", idx.abs() as usize))).unwrap()
            )
        }
    }
}
