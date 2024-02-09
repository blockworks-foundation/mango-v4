use anchor_lang::prelude::*;

use crate::accounts_ix::*;
use crate::error::MangoError;
use crate::state::*;

pub fn over_alloc(_ctx: Context<OverAlloc>) -> Result<()> {
    let mut bomb = vec![];
    let mut i = 0;
    while i < 8_000 { // 8_500 busts the heap
        bomb.push(70u8);
        i += 1;
    }
    Ok(())
}
