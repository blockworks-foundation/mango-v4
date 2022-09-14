use anchor_lang::prelude::*;
use solana_address_lookup_table_program as solana_alt;
use solana_program::pubkey::Pubkey;

pub fn addresses(table: &[u8]) -> &[Pubkey] {
    bytemuck::try_cast_slice(&table[solana_alt::state::LOOKUP_TABLE_META_SIZE..]).unwrap()
}

pub fn contains(table: &[u8], pubkey: &Pubkey) -> bool {
    addresses(table).iter().any(|&addr| addr == *pubkey)
}

pub fn cpi_extend<'info>(
    lookup_table_ai: AccountInfo<'info>,
    authority_ai: AccountInfo<'info>,
    payer_ai: AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    new_addresses: Vec<Pubkey>,
) -> std::result::Result<(), ProgramError> {
    let instruction = solana_alt::instruction::extend_lookup_table(
        lookup_table_ai.key(),
        authority_ai.key(),
        Some(payer_ai.key()),
        new_addresses,
    );
    let account_infos = [lookup_table_ai, authority_ai, payer_ai];
    solana_program::program::invoke_signed(&instruction, &account_infos, signer_seeds)
}
