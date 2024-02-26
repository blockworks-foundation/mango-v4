use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::{AnchorDeserialize, Discriminator};
use derivative::Derivative;
use fixed::types::I80F48;

use static_assertions::const_assert_eq;
use switchboard_program::FastRoundResultAccountData;
use switchboard_v2::AggregatorAccountData;

use crate::accounts_zerocopy::*;

use crate::error::*;
use crate::state::load_orca_pool_state;

use super::{load_raydium_pool_state, orca_mainnet_whirlpool, raydium_mainnet};

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
pub const SOL_DECIMALS: i8 = 9;
pub const QUOTE_NATIVE_TO_UI: I80F48 = power_of_ten(-QUOTE_DECIMALS);

pub mod switchboard_v1_devnet_oracle {
    use solana_program::declare_id;
    declare_id!("7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU");
}
pub mod switchboard_v2_mainnet_oracle {
    use solana_program::declare_id;
    declare_id!("DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM");
}

pub mod pyth_mainnet_usdc_oracle {
    use solana_program::declare_id;
    declare_id!("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD");
}

pub mod pyth_mainnet_sol_oracle {
    use solana_program::declare_id;
    declare_id!("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG");
}

pub mod usdc_mint_mainnet {
    use solana_program::declare_id;
    declare_id!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
}

pub mod sol_mint_mainnet {
    use solana_program::declare_id;
    declare_id!("So11111111111111111111111111111111111111112");
}

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, PartialEq, Eq)]
#[derivative(Debug)]
pub struct OracleConfig {
    pub conf_filter: I80F48,
    pub max_staleness_slots: i64,
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 72],
}
const_assert_eq!(size_of::<OracleConfig>(), 16 + 8 + 72);
const_assert_eq!(size_of::<OracleConfig>(), 96);
const_assert_eq!(size_of::<OracleConfig>() % 8, 0);

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Default)]
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

#[derive(Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum OracleType {
    Pyth,
    Stub,
    SwitchboardV1,
    SwitchboardV2,
    OrcaCLMM,
    RaydiumCLMM,
}

pub struct OracleState {
    pub price: I80F48,
    pub deviation: I80F48,
    pub last_update_slot: u64,
    pub oracle_type: OracleType,
}

impl OracleState {
    #[inline]
    pub fn check_confidence_and_maybe_staleness(
        &self,
        config: &OracleConfig,
        staleness_slot: Option<u64>,
    ) -> Result<()> {
        if let Some(now_slot) = staleness_slot {
            self.check_staleness(config, now_slot)?;
        }
        self.check_confidence(config)
    }

    pub fn check_staleness(&self, config: &OracleConfig, now_slot: u64) -> Result<()> {
        if config.max_staleness_slots >= 0
            && self
                .last_update_slot
                .saturating_add(config.max_staleness_slots as u64)
                < now_slot
        {
            return Err(MangoError::OracleStale.into());
        }
        Ok(())
    }

    pub fn check_confidence(&self, config: &OracleConfig) -> Result<()> {
        if self.deviation > config.conf_filter * self.price {
            return Err(MangoError::OracleConfidence.into());
        }
        Ok(())
    }
}

#[account(zero_copy)]
pub struct StubOracle {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,
    // ABI: Clients rely on this being at offset 40
    pub mint: Pubkey,
    pub price: I80F48,
    pub last_update_ts: i64,
    pub last_update_slot: u64,
    pub deviation: I80F48,
    pub reserved: [u8; 104],
}
const_assert_eq!(size_of::<StubOracle>(), 32 + 32 + 16 + 8 + 8 + 16 + 104);
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
    } else if acc_info.owner() == &orca_mainnet_whirlpool::ID {
        return Ok(OracleType::OrcaCLMM);
    } else if acc_info.owner() == &raydium_mainnet::ID {
        return Ok(OracleType::RaydiumCLMM);
    }

    Err(MangoError::UnknownOracleType.into())
}

pub fn check_is_valid_fallback_oracle(acc_info: &impl KeyedAccountReader) -> Result<()> {
    if acc_info.key() == &Pubkey::default() {
        return Ok(());
    };
    let oracle_type = determine_oracle_type(acc_info)?;
    let valid_oracle = match oracle_type {
        OracleType::OrcaCLMM => {
            let whirlpool = load_orca_pool_state(acc_info)?;
            whirlpool.has_quote_token()
        }
        OracleType::RaydiumCLMM => {
            let pool = load_raydium_pool_state(acc_info)?;
            pool.has_quote_token()
        }
        _ => true,
    };

    require!(valid_oracle, MangoError::UnexpectedOracle);
    Ok(())
}

/// Get the pyth agg price if it's available, otherwise take the prev price.
///
/// Returns the publish slot in addition to the price info.
///
/// Also see pyth's PriceAccount::get_price_no_older_than().
fn pyth_get_price(
    pubkey: &Pubkey,
    account: &pyth_sdk_solana::state::PriceAccount,
) -> (pyth_sdk_solana::Price, u64) {
    use pyth_sdk_solana::*;
    if account.agg.status == state::PriceStatus::Trading {
        (
            Price {
                conf: account.agg.conf,
                expo: account.expo,
                price: account.agg.price,
                publish_time: account.timestamp,
            },
            account.agg.pub_slot,
        )
    } else {
        (
            Price {
                conf: account.prev_conf,
                expo: account.expo,
                price: account.prev_price,
                publish_time: account.prev_timestamp,
            },
            account.prev_slot,
        )
    }
}

pub fn get_pyth_state(
    acc_info: &(impl KeyedAccountReader + ?Sized),
    base_decimals: u8,
) -> Result<OracleState> {
    let data = &acc_info.data();
    let price_account = pyth_sdk_solana::state::load_price_account(data).unwrap();
    let (price_data, last_update_slot) = pyth_get_price(acc_info.key(), price_account);

    let decimals = (price_account.expo as i8) + QUOTE_DECIMALS - (base_decimals as i8);
    let decimal_adj = power_of_ten(decimals);
    let price = I80F48::from_num(price_data.price) * decimal_adj;
    let deviation = I80F48::from_num(price_data.conf) * decimal_adj;
    require_gte!(price, 0);
    Ok(OracleState {
        price,
        last_update_slot,
        deviation,
        oracle_type: OracleType::Pyth,
    })
}

/// Contains all oracle account infos that could be used to read price
pub struct OracleAccountInfos<'a, T: KeyedAccountReader> {
    pub oracle: &'a T,
    pub fallback_opt: Option<&'a T>,
    pub usdc_opt: Option<&'a T>,
    pub sol_opt: Option<&'a T>,
}

impl<'a, T: KeyedAccountReader> OracleAccountInfos<'a, T> {
    pub fn from_reader(acc_reader: &'a T) -> Self {
        OracleAccountInfos {
            oracle: acc_reader,
            fallback_opt: None,
            usdc_opt: None,
            sol_opt: None,
        }
    }
}

/// Returns the price of one native base token, in native quote tokens
///
/// Example: The price for SOL at 40 USDC/SOL it would return 0.04 (the unit is USDC-native/SOL-native)
///
/// This currently assumes that quote decimals (i.e. decimals for USD) is 6, like for USDC.
///
/// The staleness and confidence of the oracle is not checked. Use the functions on
/// OracleState to validate them if needed. That's why this function is called _unchecked.
pub fn oracle_state_unchecked<T: KeyedAccountReader>(
    acc_infos: &OracleAccountInfos<T>,
    base_decimals: u8,
) -> Result<OracleState> {
    oracle_state_unchecked_inner(acc_infos, base_decimals, false)
}

pub fn fallback_oracle_state_unchecked<T: KeyedAccountReader>(
    acc_infos: &OracleAccountInfos<T>,
    base_decimals: u8,
) -> Result<OracleState> {
    oracle_state_unchecked_inner(acc_infos, base_decimals, true)
}

fn oracle_state_unchecked_inner<T: KeyedAccountReader>(
    acc_infos: &OracleAccountInfos<T>,
    base_decimals: u8,
    use_fallback: bool,
) -> Result<OracleState> {
    let oracle_info = if use_fallback {
        acc_infos
            .fallback_opt
            .ok_or_else(|| error!(MangoError::UnknownOracleType))?
    } else {
        acc_infos.oracle
    };
    let data = &oracle_info.data();
    let oracle_type = determine_oracle_type(oracle_info)?;

    Ok(match oracle_type {
        OracleType::Stub => {
            let stub = oracle_info.load::<StubOracle>()?;
            let deviation = if stub.deviation == 0 {
                // allows the confidence check to pass even for negative prices
                I80F48::MIN
            } else {
                stub.deviation
            };
            let last_update_slot = if stub.last_update_slot == 0 {
                // ensure staleness checks will never fail
                u64::MAX
            } else {
                stub.last_update_slot
            };
            OracleState {
                price: stub.price,
                last_update_slot,
                deviation,
                oracle_type: OracleType::Stub,
            }
        }
        OracleType::Pyth => get_pyth_state(oracle_info, base_decimals)?,
        OracleType::SwitchboardV2 => {
            fn from_foreign_error(e: impl std::fmt::Display) -> Error {
                error_msg!("{}", e)
            }

            let feed = bytemuck::from_bytes::<AggregatorAccountData>(&data[8..]);
            let feed_result = feed.get_result().map_err(from_foreign_error)?;
            let ui_price: f64 = feed_result.try_into().map_err(from_foreign_error)?;
            let ui_deviation: f64 = feed
                .latest_confirmed_round
                .std_deviation
                .try_into()
                .map_err(from_foreign_error)?;

            // The round_open_slot is an underestimate of the last update slot: Reporters will see
            // the round opening and only then start executing the price tasks.
            let last_update_slot = feed.latest_confirmed_round.round_open_slot;

            let decimals = QUOTE_DECIMALS - (base_decimals as i8);
            let decimal_adj = power_of_ten(decimals);
            let price = I80F48::from_num(ui_price) * decimal_adj;
            let deviation = I80F48::from_num(ui_deviation) * decimal_adj;
            require_gte!(price, 0);
            OracleState {
                price,
                last_update_slot,
                deviation,
                oracle_type: OracleType::SwitchboardV2,
            }
        }
        OracleType::SwitchboardV1 => {
            let result = FastRoundResultAccountData::deserialize(data).unwrap();
            let ui_price = I80F48::from_num(result.result.result);

            let ui_deviation =
                I80F48::from_num(result.result.max_response - result.result.min_response);
            let last_update_slot = result.result.round_open_slot;

            let decimals = QUOTE_DECIMALS - (base_decimals as i8);
            let decimal_adj = power_of_ten(decimals);
            let price = ui_price * decimal_adj;
            let deviation = ui_deviation * decimal_adj;
            require_gte!(price, 0);
            OracleState {
                price,
                last_update_slot,
                deviation,
                oracle_type: OracleType::SwitchboardV1,
            }
        }
        OracleType::OrcaCLMM => {
            let whirlpool = load_orca_pool_state(oracle_info)?;
            let clmm_price = whirlpool.get_clmm_price();
            let quote_oracle_state = whirlpool.quote_state_unchecked(acc_infos)?;
            let price = clmm_price * quote_oracle_state.price;
            OracleState {
                price,
                last_update_slot: quote_oracle_state.last_update_slot,
                deviation: quote_oracle_state.deviation,
                oracle_type: OracleType::OrcaCLMM,
            }
        }
        OracleType::RaydiumCLMM => {
            let whirlpool = load_raydium_pool_state(oracle_info)?;
            let clmm_price = whirlpool.get_clmm_price();
            let quote_oracle_state = whirlpool.quote_state_unchecked(acc_infos)?;
            let price = clmm_price * quote_oracle_state.price;
            OracleState {
                price,
                last_update_slot: quote_oracle_state.last_update_slot,
                deviation: quote_oracle_state.deviation,
                oracle_type: OracleType::RaydiumCLMM,
            }
        }
    })
}

pub fn oracle_log_context(
    name: &str,
    state: &OracleState,
    oracle_config: &OracleConfig,
    staleness_slot: Option<u64>,
) -> String {
    format!(
        "name: {}, price: {}, deviation: {}, last_update_slot: {}, now_slot: {}, conf_filter: {:#?}",
        name,
        state.price.to_num::<f64>(),
        state.deviation.to_num::<f64>(),
        state.last_update_slot,
        staleness_slot.unwrap_or_else(|| u64::MAX),
        oracle_config.conf_filter.to_num::<f32>(),
    )
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
            (
                "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d",
                OracleType::OrcaCLMM,
                orca_mainnet_whirlpool::ID,
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

    #[test]
    pub fn test_clmm_prices() -> Result<()> {
        // add ability to find fixtures
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");

        let usdc_fixture = (
            "Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD",
            OracleType::Pyth,
            Pubkey::default(),
            6,
        );

        let clmm_fixtures = vec![
            (
                "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d",
                OracleType::OrcaCLMM,
                orca_mainnet_whirlpool::ID,
                9, // SOL/USDC pool
            ),
            (
                "Ds33rQ1d4AXwxqyeXX6Pc3G4pFNr6iWb3dd8YfBBQMPr",
                OracleType::RaydiumCLMM,
                raydium_mainnet::ID,
                9, // SOL/USDC pool
            ),
        ];

        for fixture in clmm_fixtures {
            let clmm_file = format!("resources/test/{}.bin", fixture.0);
            let mut clmm_data = read_file(find_file(&clmm_file).unwrap());
            let data = RefCell::new(&mut clmm_data[..]);
            let ai = &AccountInfoRef {
                key: &Pubkey::from_str(fixture.0).unwrap(),
                owner: &fixture.2,
                data: data.borrow(),
            };

            let pyth_file = format!("resources/test/{}.bin", usdc_fixture.0);
            let mut pyth_data = read_file(find_file(&pyth_file).unwrap());
            let pyth_data_cell = RefCell::new(&mut pyth_data[..]);
            let usdc_ai = &AccountInfoRef {
                key: &Pubkey::from_str(usdc_fixture.0).unwrap(),
                owner: &usdc_fixture.2,
                data: pyth_data_cell.borrow(),
            };
            let base_decimals = fixture.3;
            let usdc_decimals = usdc_fixture.3;

            let usdc_ais = OracleAccountInfos {
                oracle: usdc_ai,
                fallback_opt: None,
                usdc_opt: None,
                sol_opt: None,
            };
            let clmm_ais = OracleAccountInfos {
                oracle: ai,
                fallback_opt: None,
                usdc_opt: Some(usdc_ai),
                sol_opt: None,
            };
            let usdc = oracle_state_unchecked(&usdc_ais, usdc_decimals).unwrap();
            let clmm = oracle_state_unchecked(&clmm_ais, base_decimals).unwrap();
            assert!(usdc.price == I80F48::from_num(1.00000758274099));

            match fixture.1 {
                OracleType::OrcaCLMM => {
                    // 63.006792786538313 * 1.00000758274099 (but in native/native)
                    assert!(clmm.price == I80F48::from_num(0.06300727055072872))
                }
                OracleType::RaydiumCLMM => {
                    // 83.551469620431 * 1.00000758274099 (but in native/native)
                    assert!(clmm.price == I80F48::from_num(0.083552103169584))
                }
                _ => unimplemented!(),
            }
        }
        Ok(())
    }

    #[test]
    pub fn test_clmm_price_missing_usdc() -> Result<()> {
        // add ability to find fixtures
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");

        let fixtures = vec![
            (
                "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d",
                OracleType::OrcaCLMM,
                orca_mainnet_whirlpool::ID,
                9, // SOL/USDC pool
            ),
            (
                "Ds33rQ1d4AXwxqyeXX6Pc3G4pFNr6iWb3dd8YfBBQMPr",
                OracleType::RaydiumCLMM,
                raydium_mainnet::ID,
                9, // SOL/USDC pool
            ),
        ];

        for fixture in fixtures {
            let filename = format!("resources/test/{}.bin", fixture.0);
            let mut clmm_data = read_file(find_file(&filename).unwrap());
            let data = RefCell::new(&mut clmm_data[..]);
            let ai = &AccountInfoRef {
                key: &Pubkey::from_str(fixture.0).unwrap(),
                owner: &fixture.2,
                data: data.borrow(),
            };
            let base_decimals = fixture.3;
            assert!(determine_oracle_type(ai).unwrap() == fixture.1);
            let oracle_infos = OracleAccountInfos {
                oracle: ai,
                fallback_opt: None,
                usdc_opt: None,
                sol_opt: None,
            };
            assert!(oracle_state_unchecked(&oracle_infos, base_decimals)
                .is_anchor_error_with_code(6068));
        }

        Ok(())
    }

    #[test]
    pub fn test_valid_fallbacks() -> Result<()> {
        // add ability to find fixtures
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");

        let usdc_fixture = (
            "Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD",
            OracleType::Pyth,
            Pubkey::default(),
            6,
        );

        let clmm_fixtures = vec![
            (
                "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d",
                OracleType::OrcaCLMM,
                orca_mainnet_whirlpool::ID,
                9, // SOL/USDC pool
            ),
            (
                "Ds33rQ1d4AXwxqyeXX6Pc3G4pFNr6iWb3dd8YfBBQMPr",
                OracleType::RaydiumCLMM,
                raydium_mainnet::ID,
                9, // SOL/USDC pool
            ),
        ];

        for fixture in clmm_fixtures {
            let clmm_file = format!("resources/test/{}.bin", fixture.0);
            let mut clmm_data = read_file(find_file(&clmm_file).unwrap());
            let data = RefCell::new(&mut clmm_data[..]);
            let ai = &AccountInfoRef {
                key: &Pubkey::from_str(fixture.0).unwrap(),
                owner: &fixture.2,
                data: data.borrow(),
            };

            check_is_valid_fallback_oracle(ai)?;
        }
        Ok(())
    }
}
