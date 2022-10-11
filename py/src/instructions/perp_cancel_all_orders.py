from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class PerpCancelAllOrdersArgs(typing.TypedDict):
    limit: int


layout = borsh.CStruct("limit" / borsh.U8)


class PerpCancelAllOrdersAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    perp_market: PublicKey
    asks: PublicKey
    bids: PublicKey


def perp_cancel_all_orders(
    args: PerpCancelAllOrdersArgs,
    accounts: PerpCancelAllOrdersAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["perp_market"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["asks"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["bids"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"`\x10\xe2\xb5k\x91\xe0\xd5"
    encoded_args = layout.build(
        {
            "limit": args["limit"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
