from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from ..program_id import PROGRAM_ID


class Serum3SettleFundsAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    open_orders: PublicKey
    serum_market: PublicKey
    serum_program: PublicKey
    serum_market_external: PublicKey
    market_base_vault: PublicKey
    market_quote_vault: PublicKey
    market_vault_signer: PublicKey
    quote_bank: PublicKey
    quote_vault: PublicKey
    base_bank: PublicKey
    base_vault: PublicKey
    token_program: PublicKey


def serum3_settle_funds(
    accounts: Serum3SettleFundsAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["open_orders"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["serum_market"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_market_external"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_base_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_quote_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_vault_signer"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["quote_bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["quote_vault"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["base_bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["base_vault"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"U<i\xe5\xe23%k"
    encoded_args = b""
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
