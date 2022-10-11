from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class AccountCreateArgs(typing.TypedDict):
    account_num: int
    token_count: int
    serum3_count: int
    perp_count: int
    perp_oo_count: int
    name: str


layout = borsh.CStruct(
    "account_num" / borsh.U32,
    "token_count" / borsh.U8,
    "serum3_count" / borsh.U8,
    "perp_count" / borsh.U8,
    "perp_oo_count" / borsh.U8,
    "name" / borsh.String,
)


class AccountCreateAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    payer: PublicKey
    system_program: PublicKey


def account_create(
    args: AccountCreateArgs,
    accounts: AccountCreateAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["payer"], is_signer=True, is_writable=True),
        AccountMeta(
            pubkey=accounts["system_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xc6_'\xc5)\xd6\x9d\x12"
    encoded_args = layout.build(
        {
            "account_num": args["account_num"],
            "token_count": args["token_count"],
            "serum3_count": args["serum3_count"],
            "perp_count": args["perp_count"],
            "perp_oo_count": args["perp_oo_count"],
            "name": args["name"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
