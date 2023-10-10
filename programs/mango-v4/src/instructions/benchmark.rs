use anchor_lang::prelude::*;
use fixed::types::{I80F48, U80F48};
use solana_program::log::sol_log_compute_units;

use crate::accounts_ix::*;
use crate::i80f48::LowPrecisionDivision;

#[inline(never)]
pub fn run_bench<T: std::fmt::Debug>(name: &str, fun: impl Fn() -> T) {
    msg!("BENCH: {}", name);
    sol_log_compute_units();
    let r = fun();
    sol_log_compute_units();
    msg! {"{:?}", r}
}

pub fn benchmark(_ctx: Context<Benchmark>) -> Result<()> {
    run_bench("nothing", || {});

    let clock = Clock::get().unwrap();

    let s = 71 + clock.slot as i64;
    let t = 42 + clock.unix_timestamp as i64;

    {
        let s = s as i128;
        let t = t as i128;
        let a = I80F48::from_bits((s << 64) + t);
        let b = I80F48::from_bits((t << 64) + s);
        run_bench("division_i80f48", || a.checked_div(b).unwrap());
        run_bench("division_i128", || {
            a.to_bits().checked_div(b.to_bits()).unwrap()
        });
        run_bench("division_i80f48_30bit", || {
            a.checked_div_30bit_precision(b).unwrap()
        });
        run_bench("division_i80f48_f64", || {
            a.checked_div_f64_precision(b).unwrap()
        });
        run_bench("conversion_i80f48_to_f64", || a.to_num::<f64>());
        let f = a.to_num::<f64>();
        run_bench("conversion_f64_to_i80f48", || I80F48::from_num(f));
        let a2: I80F48 = a >> 64;
        let b2: I80F48 = b >> 64;
        run_bench("add_i80f48", || a2.checked_add(b2).unwrap());
        run_bench("mul_i80f48", || a2.checked_mul(b2).unwrap());
    }

    {
        let s = s as u128;
        let t = t as u128;
        let a = U80F48::from_bits((s << 64) + t);
        let b = U80F48::from_bits((t << 64) + s);
        run_bench("division_u80f48", || a.checked_div(b).unwrap());
    }

    {
        let a = s as i64;
        let b = t as i64;
        run_bench("division_i64", || a.checked_div(b).unwrap());
    }

    {
        let a = s as i32;
        let b = t as i32;
        run_bench("division_i32", || a.checked_div(b).unwrap());
    }

    {
        let a = s as f64;
        let b = t as f64;
        run_bench("add_f64", || a + b);
        run_bench("mul_f64", || a * b);
        run_bench("division_f64", || a / b);
    }

    {
        let a = s as f32;
        let b = t as f32;
        run_bench("add_f32", || a + b);
        run_bench("mul_f32", || a * b);
        run_bench("division_f32", || a / b);
    }

    {
        let a = s as u32;
        let b = t as u32;
        run_bench("add_u32", || a + b);
        run_bench("division_u32", || a.checked_div(b).unwrap());
    }
    {
        let a = s as u64;
        let b = t as u64;
        run_bench("add_u64", || a + b);
        run_bench("division_u64", || a.checked_div(b).unwrap());
    }

    Ok(())
}
