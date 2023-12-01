use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::state::*;

pub fn over_alloc(_ctx: Context<OverAlloc>) -> Result<()> {
    let mut bomb = vec![];
    let mut i = 0;
    while i < 50 * 1024 {
        bomb.push(70u8);
        i += 1;
    }
    Ok(())
}
