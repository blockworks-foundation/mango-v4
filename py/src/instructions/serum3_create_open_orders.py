from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from ..program_id import PROGRAM_ID


class Serum3CreateOpenOrdersAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    serum_market: PublicKey
    serum_program: PublicKey
    serum_market_external: PublicKey
    open_orders: PublicKey
    payer: PublicKey
    system_program: PublicKey
    rent: PublicKey


def serum3_create_open_orders(
    accounts: Serum3CreateOpenOrdersAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["serum_market"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_market_external"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["open_orders"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["payer"], is_signer=True, is_writable=True),
        AccountMeta(
            pubkey=accounts["system_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["rent"], is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\x04\xf4#(US\xcb\xfa"
    encoded_args = b""
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
