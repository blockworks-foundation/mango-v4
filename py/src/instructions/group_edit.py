from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class GroupEditArgs(typing.TypedDict):
    admin_opt: typing.Optional[PublicKey]
    fast_listing_admin_opt: typing.Optional[PublicKey]
    testing_opt: typing.Optional[int]
    version_opt: typing.Optional[int]


layout = borsh.CStruct(
    "admin_opt" / borsh.Option(BorshPubkey),
    "fast_listing_admin_opt" / borsh.Option(BorshPubkey),
    "testing_opt" / borsh.Option(borsh.U8),
    "version_opt" / borsh.Option(borsh.U8),
)


class GroupEditAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey


def group_edit(
    args: GroupEditArgs,
    accounts: GroupEditAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\x08X\xb7\xf9\xa6s7\xe3"
    encoded_args = layout.build(
        {
            "admin_opt": args["admin_opt"],
            "fast_listing_admin_opt": args["fast_listing_admin_opt"],
            "testing_opt": args["testing_opt"],
            "version_opt": args["version_opt"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
