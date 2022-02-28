use std::cell::Ref;

use crate::error::MangoError;
use anchor_lang::__private::bytemuck::{Pod, Zeroable};
use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::Mango;

pub enum OracleType {
    Stub,
}

// TODO: what name would be better - stub or mock or integration test oracle?
#[account(zero_copy)]
pub struct StubOracle {
    pub magic: u32,
    pub price: I80F48,
    pub last_updated: i64,
}

pub fn determine_oracle_type(account: &AccountInfo) -> Result<OracleType> {
    let borrowed = &account.data.borrow();
    if borrowed[0] == 224 && borrowed[1] == 251 && borrowed[2] == 254 && borrowed[3] == 99 {
        return Ok(OracleType::Stub);
    }
    Err(MangoError::UnknownOracle.into())
}
