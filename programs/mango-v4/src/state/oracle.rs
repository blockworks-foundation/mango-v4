use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::{AnchorDeserialize, Discriminator};
use derivative::Derivative;
use fixed::types::{I80F48, U64F64};

use static_assertions::const_assert_eq;
use switchboard_program::FastRoundResultAccountData;
use switchboard_v2::AggregatorAccountData;

use crate::accounts_zerocopy::*;

use crate::error::*;

use super::{orca_mainnet_whirlpool, Whirlpool};

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

pub mod pyth_mainnet_usdc_oracle {
    use solana_program::declare_id;
    declare_id!("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD");
}

pub mod usdc_mint_mainnet {
    use solana_program::declare_id;
    declare_id!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
}

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative)]
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

#[derive(Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum OracleType {
    Pyth,
    Stub,
    SwitchboardV1,
    SwitchboardV2,
    OrcaCLMM,
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
    }

    Err(MangoError::UnknownOracleType.into())
}

pub fn check_is_valid_fallback_oracle(acc_info: &impl KeyedAccountReader) -> Result<()> {
    if acc_info.key() == &Pubkey::default() {
        return Ok(());
    };
    let oracle_type = determine_oracle_type(acc_info)?;
    if oracle_type == OracleType::OrcaCLMM {
        let data = &acc_info.data();
        let whirlpool = Whirlpool::try_deserialize(&mut &data[..]).unwrap();
        require!(
            whirlpool.token_mint_a == usdc_mint_mainnet::ID
                || whirlpool.token_mint_b == usdc_mint_mainnet::ID,
            MangoError::InvalidCLMMOracle
        );
    }
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

fn get_pyth_state(
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

/// Returns the price of one native base token, in native quote tokens
///
/// Example: The price for SOL at 40 USDC/SOL it would return 0.04 (the unit is USDC-native/SOL-native)
///
/// This currently assumes that quote decimals (i.e. decimals for USD) is 6, like for USDC.
///
/// The staleness and confidence of the oracle is not checked. Use the functions on
/// OracleState to validate them if needed. That's why this function is called _unchecked.
pub fn oracle_state_unchecked(
    acc_info: &impl KeyedAccountReader,
    usd_feed_opt: Option<&(impl KeyedAccountReader + ?Sized)>,
    base_decimals: u8,
) -> Result<OracleState> {
    let data = &acc_info.data();
    let oracle_type = determine_oracle_type(acc_info)?;

    Ok(match oracle_type {
        OracleType::Stub => {
            let stub = acc_info.load::<StubOracle>()?;
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
        OracleType::Pyth => get_pyth_state(acc_info, base_decimals)?,
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
            let usd_state = usdc_state_unchecked(usd_feed_opt)?;

            let whirlpool = Whirlpool::try_deserialize(&mut &data[..]).unwrap();

            let sqrt_price = U64F64::from_bits(whirlpool.sqrt_price);

            let clmm_price = if whirlpool.token_mint_a == usdc_mint_mainnet::ID {
                let inverted_price = I80F48::from_num(sqrt_price * sqrt_price);
                Ok(I80F48::ONE.checked_div(inverted_price).unwrap())
            } else if whirlpool.token_mint_b == usdc_mint_mainnet::ID {
                Ok(I80F48::from_num(sqrt_price * sqrt_price))
            } else {
                Err(MangoError::InvalidCLMMOracle)
            }?;

            let price = clmm_price * usd_state.price;
            OracleState {
                price,
                last_update_slot: usd_state.last_update_slot,
                deviation: I80F48::ZERO,
                oracle_type: OracleType::OrcaCLMM,
            }
        }
    })
}

#[inline(always)]
fn usdc_state_unchecked(
    usd_feed_opt: Option<&(impl KeyedAccountReader + ?Sized)>,
) -> Result<OracleState> {
    if usd_feed_opt.is_none() {
        return Err(MangoError::MissingFeedForCLMMOracle.into());
    }
    let usd_feed = usd_feed_opt.unwrap();
    if usd_feed.key() != &pyth_mainnet_usdc_oracle::ID {
        return Err(MangoError::InvalidFeedForCLMMOracle.into());
    }
    let usd_state = get_pyth_state(usd_feed, QUOTE_DECIMALS as u8)?;
    Ok(usd_state)
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
    use crate::health::EMPTY_KEYED_READER_OPT;

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
    pub fn test_clmm_price() -> Result<()> {
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
                "Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD",
                OracleType::Pyth,
                Pubkey::default(),
                6,
            ),
        ];

        let clmm_file = format!("resources/test/{}.bin", fixtures[0].0);
        let mut clmm_data = read_file(find_file(&clmm_file).unwrap());
        let data = RefCell::new(&mut clmm_data[..]);
        let ai = &AccountInfoRef {
            key: &Pubkey::from_str(fixtures[0].0).unwrap(),
            owner: &fixtures[0].2,
            data: data.borrow(),
        };

        let pyth_file = format!("resources/test/{}.bin", fixtures[1].0);
        let mut pyth_data = read_file(find_file(&pyth_file).unwrap());
        let pyth_data_cell = RefCell::new(&mut pyth_data[..]);
        let usdc_ai = &AccountInfoRef {
            key: &Pubkey::from_str(fixtures[1].0).unwrap(),
            owner: &fixtures[1].2,
            data: pyth_data_cell.borrow(),
        };
        let base_decimals = fixtures[0].3;
        let usdc_decimals = fixtures[1].3;

        let usdc = oracle_state_unchecked(usdc_ai, EMPTY_KEYED_READER_OPT, usdc_decimals).unwrap();
        let orca = oracle_state_unchecked(ai, Some(usdc_ai), base_decimals).unwrap();
        assert!(usdc.price == I80F48::from_num(1.00000758274099));
        // 63.006792786538313 * 1.00000758274099 (but in native/native)
        assert!(orca.price == I80F48::from_num(0.06300727055072872));

        Ok(())
    }

    #[test]
    pub fn test_clmm_price_missing_usdc() -> Result<()> {
        // add ability to find fixtures
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");

        let fixtures = vec![(
            "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d",
            OracleType::OrcaCLMM,
            orca_mainnet_whirlpool::ID,
            9, // SOL/USDC pool
        )];

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
            assert!(
                oracle_state_unchecked(ai, EMPTY_KEYED_READER_OPT, base_decimals)
                    .is_anchor_error_with_code(6065)
            );
        }

        Ok(())
    }
}
