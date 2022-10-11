from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class PerpCancelAllOrdersBySideArgs(typing.TypedDict):
    side_option: typing.Optional[types.side.SideKind]
    limit: int


layout = borsh.CStruct(
    "side_option" / borsh.Option(types.side.layout), "limit" / borsh.U8
)


class PerpCancelAllOrdersBySideAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    perp_market: PublicKey
    asks: PublicKey
    bids: PublicKey


def perp_cancel_all_orders_by_side(
    args: PerpCancelAllOrdersBySideArgs,
    accounts: PerpCancelAllOrdersBySideAccounts,
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
    identifier = b"3\xf8\xcc}e\xb6k\x92"
    encoded_args = layout.build(
        {
            "side_option": (
                None
                if args["side_option"] is None
                else args["side_option"].to_encodable()
            ),
            "limit": args["limit"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
