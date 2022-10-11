from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class AltSetArgs(typing.TypedDict):
    index: int


layout = borsh.CStruct("index" / borsh.U8)


class AltSetAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey
    address_lookup_table: PublicKey


def alt_set(
    args: AltSetArgs,
    accounts: AltSetAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["address_lookup_table"], is_signer=False, is_writable=True
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xebD\x91 ;i7\x19"
    encoded_args = layout.build(
        {
            "index": args["index"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
