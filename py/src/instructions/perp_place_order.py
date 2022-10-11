from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class PerpPlaceOrderArgs(typing.TypedDict):
    side: types.side.SideKind
    price_lots: int
    max_base_lots: int
    max_quote_lots: int
    client_order_id: int
    order_type: types.order_type.OrderTypeKind
    expiry_timestamp: int
    limit: int


layout = borsh.CStruct(
    "side" / types.side.layout,
    "price_lots" / borsh.I64,
    "max_base_lots" / borsh.I64,
    "max_quote_lots" / borsh.I64,
    "client_order_id" / borsh.U64,
    "order_type" / types.order_type.layout,
    "expiry_timestamp" / borsh.U64,
    "limit" / borsh.U8,
)


class PerpPlaceOrderAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    perp_market: PublicKey
    asks: PublicKey
    bids: PublicKey
    event_queue: PublicKey
    oracle: PublicKey


def perp_place_order(
    args: PerpPlaceOrderArgs,
    accounts: PerpPlaceOrderAccounts,
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
        AccountMeta(pubkey=accounts["event_queue"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xbd\xc4\xe1\xc9r\xac\x19\xa6"
    encoded_args = layout.build(
        {
            "side": args["side"].to_encodable(),
            "price_lots": args["price_lots"],
            "max_base_lots": args["max_base_lots"],
            "max_quote_lots": args["max_quote_lots"],
            "client_order_id": args["client_order_id"],
            "order_type": args["order_type"].to_encodable(),
            "expiry_timestamp": args["expiry_timestamp"],
            "limit": args["limit"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
