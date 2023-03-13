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

    let clock = Clock::get().unwrap();

    {
        let s = clock.slot as i128;
        let t = clock.unix_timestamp as i128;
        let a = I80F48::from_bits((s << 64) + s);
        let b = I80F48::from_bits((t << 64) + t);
        division_i80f48(a, b); // 1000 - 5000 CU
        division_i128(a.to_bits(), b.to_bits()); // 100 - 2000 CU
        division_i80f48_30bit(a, b); // 300 CU
        division_i80f48_f64(a, b); // 500 CU
        mul_i80f48(a >> 64, b >> 64); // 100 CU
        i80f48_to_f64(a); // 50 CU
    }

    {
        let s = clock.slot as u128;
        let t = clock.unix_timestamp as u128;
        let a = U80F48::from_bits((s << 64) + s);
        let b = U80F48::from_bits((t << 64) + t);
        division_u80f48(a, b); // 1000 - 5000 CU
    }

    {
        let a = clock.slot as i64;
        let b = clock.unix_timestamp as i64;
        division_i64(a, b); // 50 CU
    }

    {
        let a = clock.slot as i32;
        let b = clock.unix_timestamp as i32;
        division_i32(a, b); // 50 CU
    }

    {
        let a = clock.slot as u32;
        let b = clock.unix_timestamp as u32;
        division_u32(a, b); // 20 CU
    }

    {
        let a = clock.slot as f64;
        let b = clock.unix_timestamp as f64;
        mul_f64(a, b); // 0 CU??
    }

    sol_log_compute_units(); // 100321 -> 101
    msg!("msg!"); // 100079+101 -> 203
    sol_log_compute_units(); // 100117

    let pk1 = Pubkey::default(); // 10
    sol_log_compute_units(); // 100006
    let pk2 = Pubkey::default(); // 10
    sol_log_compute_units(); // 99895

    let _ = pk1 == pk2; // 56
    sol_log_compute_units(); // 99739

    let _ = sol_memcmp(&pk1.to_bytes(), &pk2.to_bytes(), 32); // 64
    sol_log_compute_units(); // 99574

    let large_number = I80F48::from_str("777472127991.999999999999996").unwrap();
    let half = I80F48::MAX / 2;
    let max = I80F48::MAX;
    sol_log_compute_units(); // 92610

    let _ = half + half; // 0
    sol_log_compute_units(); // 92509

    let _ = max - max; // 0
    sol_log_compute_units(); // 92408

    let _ = large_number * large_number; // 77
    sol_log_compute_units(); // 92230

    // /
    let _ = I80F48::ZERO / max; // 839
    sol_log_compute_units(); // 91290

    let _ = half / max; // 3438
    sol_log_compute_units(); // 87751

    let _ = max / max; // 3457
    sol_log_compute_units(); // 84193

    Ok(())
}
