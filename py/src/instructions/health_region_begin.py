from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from ..program_id import PROGRAM_ID


class HealthRegionBeginAccounts(typing.TypedDict):
    instructions: PublicKey
    account: PublicKey


def health_region_begin(
    accounts: HealthRegionBeginAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["instructions"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"=C5\xc6\x8b\x84\xd3,"
    encoded_args = b""
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
