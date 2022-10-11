from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from anchorpy.borsh_extension import BorshPubkey
from construct import Construct
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class AltExtendArgs(typing.TypedDict):
    index: int
    new_addresses: list[PublicKey]


layout = borsh.CStruct(
    "index" / borsh.U8, "new_addresses" / borsh.Vec(typing.cast(Construct, BorshPubkey))
)


class AltExtendAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey
    payer: PublicKey
    address_lookup_table: PublicKey


def alt_extend(
    args: AltExtendArgs,
    accounts: AltExtendAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["payer"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["address_lookup_table"], is_signer=False, is_writable=True
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"|3/ZDB\x19b"
    encoded_args = layout.build(
        {
            "index": args["index"],
            "new_addresses": args["new_addresses"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
