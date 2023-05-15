use anchor_lang::prelude::*;
use fixed::types::{I80F48, U80F48};
use solana_program::{log::sol_log_compute_units, program_memory::sol_memcmp};
use std::str::FromStr;

use crate::accounts_ix::*;
use crate::i80f48::LowPrecisionDivision;

#[inline(never)]
pub fn division_i80f48(a: I80F48, b: I80F48) -> I80F48 {
    msg!("division_i80f48");
    sol_log_compute_units();
    let r = a.checked_div(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn division_u80f48(a: U80F48, b: U80F48) -> U80F48 {
    msg!("division_u80f48");
    sol_log_compute_units();
    let r = a.checked_div(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn division_i128(a: i128, b: i128) -> i128 {
    msg!("division_i128");
    sol_log_compute_units();
    let r = a.checked_div(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn division_i64(a: i64, b: i64) -> i64 {
    msg!("division_i64");
    sol_log_compute_units();
    let r = a.checked_div(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn division_i32(a: i32, b: i32) -> i32 {
    msg!("division_i32");
    sol_log_compute_units();
    let r = a.checked_div(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn division_u32(a: u32, b: u32) -> u32 {
    msg!("division_u32");
    sol_log_compute_units();
    let r = a.checked_div(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn division_i80f48_30bit(a: I80F48, b: I80F48) -> I80F48 {
    msg!("division_i80f48_30bit");
    sol_log_compute_units();
    let r = a.checked_div_30bit_precision(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn division_i80f48_f64(a: I80F48, b: I80F48) -> I80F48 {
    msg!("division_i80f48_f64");
    sol_log_compute_units();
    let r = a.checked_div_f64_precision(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn mul_f64(a: f64, b: f64) -> f64 {
    msg!("mul_f64");
    sol_log_compute_units();
    let r = a * b;
    if r.is_nan() {
        panic!("nan"); // here as a side-effect to avoid reordering
    }
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn mul_i80f48(a: I80F48, b: I80F48) -> I80F48 {
    msg!("mul_i80f48");
    sol_log_compute_units();
    let r = a.checked_mul(b).unwrap();
    sol_log_compute_units();
    r
}

#[inline(never)]
pub fn i80f48_to_f64(a: I80F48) -> f64 {
    msg!("i80f48_to_f64");
    sol_log_compute_units();
    let r = a.to_num::<f64>();
    if r.is_nan() {
        panic!("nan"); // here as a side-effect to avoid reordering
    }
    sol_log_compute_units();
    r
}

pub fn benchmark(_ctx: Context<Benchmark>) -> Result<()> {
    // 101000
    // 477
    sol_log_compute_units(); // 100422
    sol_log_compute_units(); // 100321 -> 101

    use crate::state::*;
    use bytemuck::Zeroable;

    // some example event queue
    let mut event_queue = _ctx.accounts.dummy.load_init()?;
    for i in 0..488 {
        let event = OutEvent::new(
            Side::Bid,
            0,
            0,
            event_queue.header.seq_num,
            Pubkey::from([i as u8; 32]),
            i,
        );
        event_queue.push_back(bytemuck::cast(event)).unwrap();
    }
    let target = Pubkey::from([1u8; 32]);
    let t: &[u8] = target.as_ref();

    sol_log_compute_units(); // 100422

    // find all events for a key
    let mut founds = 0;
    for i in 0..488 {
        let ev = &event_queue.buf[i];
        if ev.event_type == EventType::Out as u8 {
            let outev: &OutEvent = bytemuck::cast_ref(ev);
            let r: &[u8] = outev.owner.as_ref();
            if r[0..8] == t[0..8] {
                if r == t {
                    founds += 1;
                }
            }
        }
    }

    sol_log_compute_units(); // 100422

    msg!("{}", founds);

    Ok(())
}
