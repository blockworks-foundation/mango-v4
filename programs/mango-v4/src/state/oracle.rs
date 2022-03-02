use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use fixed::types::I80F48;

use crate::error::MangoError;

#[derive(PartialEq)]
pub enum OracleType {
    Stub,
    Pyth,
}

#[account(zero_copy)]
pub struct StubOracle {
    pub price: I80F48,
    pub last_updated: i64,
}

pub fn determine_oracle_type(data: &[u8]) -> Result<OracleType> {
    if u32::from_le_bytes(data[0..4].try_into().unwrap()) == pyth_client::MAGIC {
        return Ok(OracleType::Pyth);
    } else if data[0..8] == StubOracle::discriminator() {
        return Ok(OracleType::Stub);
    }

    Err(MangoError::UnknownOracleType.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyth_client::load_price;
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
        let price = load_price(pyth_price_data.as_slice()).unwrap();
        assert_eq!(price.valid_slot, 64338667);
        assert_eq!(price.agg.price, 32112500000);

        Ok(())
    }
}
