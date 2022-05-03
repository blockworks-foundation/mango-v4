use std::str::FromStr;

use anchor_lang::prelude::*;
use fixed::types::I80F48;
use solana_program::{log::sol_log_compute_units, program_memory::sol_memcmp};

#[derive(Accounts)]
pub struct Benchmark {}

pub fn benchmark(_ctx: Context<Benchmark>) -> Result<()> {
    // 101000
    // 477
    sol_log_compute_units(); // 100422

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

    let _ = checked_math!(half + half); // 0
    sol_log_compute_units(); // 92509

    let _ = checked_math!(max - max); // 0
    sol_log_compute_units(); // 92408

    let _ = checked_math!(large_number * large_number); // 77
    sol_log_compute_units(); // 92230

    // /
    let _ = checked_math!(I80F48::ZERO / max); // 839
    sol_log_compute_units(); // 91290

    let _ = checked_math!(half / max); // 3438
    sol_log_compute_units(); // 87751

    let _ = checked_math!(max / max); // 3457
    sol_log_compute_units(); // 84193

    Ok(())
}
