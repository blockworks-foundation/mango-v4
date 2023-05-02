use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Benchmark<'info> {
    /// CHECK: unused, exists only because anchor is unhappy in no-entrypoint mode otherwise
    pub dummy: UncheckedAccount<'info>,
}
