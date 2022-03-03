use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;

pub fn to_account_meta(account_info: &AccountInfo) -> AccountMeta {
    if account_info.is_writable {
        AccountMeta::new(*account_info.key, account_info.is_signer)
    } else {
        AccountMeta::new_readonly(*account_info.key, account_info.is_signer)
    }
}
