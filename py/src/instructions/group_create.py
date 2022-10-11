from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class GroupCreateArgs(typing.TypedDict):
    group_num: int
    testing: int
    version: int


layout = borsh.CStruct(
    "group_num" / borsh.U32, "testing" / borsh.U8, "version" / borsh.U8
)


class GroupCreateAccounts(typing.TypedDict):
    group: PublicKey
    creator: PublicKey
    insurance_mint: PublicKey
    insurance_vault: PublicKey
    payer: PublicKey
    token_program: PublicKey
    system_program: PublicKey
    rent: PublicKey


def group_create(
    args: GroupCreateArgs,
    accounts: GroupCreateAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["creator"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["insurance_mint"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["insurance_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=accounts["payer"], is_signer=True, is_writable=True),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["system_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["rent"], is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xe2R\xefwk\x88\xa6\xf0"
    encoded_args = layout.build(
        {
            "group_num": args["group_num"],
            "testing": args["testing"],
            "version": args["version"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
