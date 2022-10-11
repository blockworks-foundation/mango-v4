from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class AccountExpandArgs(typing.TypedDict):
    token_count: int
    serum3_count: int
    perp_count: int
    perp_oo_count: int


layout = borsh.CStruct(
    "token_count" / borsh.U8,
    "serum3_count" / borsh.U8,
    "perp_count" / borsh.U8,
    "perp_oo_count" / borsh.U8,
)


class AccountExpandAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    payer: PublicKey
    system_program: PublicKey


def account_expand(
    args: AccountExpandArgs,
    accounts: AccountExpandAccounts,
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
    identifier = b"X\xd4\x1ft\xfd\xc9Q\x01"
    encoded_args = layout.build(
        {
            "token_count": args["token_count"],
            "serum3_count": args["serum3_count"],
            "perp_count": args["perp_count"],
            "perp_oo_count": args["perp_oo_count"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
