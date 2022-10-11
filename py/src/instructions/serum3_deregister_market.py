from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from ..program_id import PROGRAM_ID


class Serum3DeregisterMarketAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey
    serum_market: PublicKey
    index_reservation: PublicKey
    sol_destination: PublicKey
    token_program: PublicKey


def serum3_deregister_market(
    accounts: Serum3DeregisterMarketAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["serum_market"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["index_reservation"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["sol_destination"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\x11\xa4*\xde\x97\xa0\x18\xb5"
    encoded_args = b""
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
