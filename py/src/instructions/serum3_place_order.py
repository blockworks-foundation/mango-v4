from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class Serum3PlaceOrderArgs(typing.TypedDict):
    side: types.serum3_side.Serum3SideKind
    limit_price: int
    max_base_qty: int
    max_native_quote_qty_including_fees: int
    self_trade_behavior: types.serum3_self_trade_behavior.Serum3SelfTradeBehaviorKind
    order_type: types.serum3_order_type.Serum3OrderTypeKind
    client_order_id: int
    limit: int


layout = borsh.CStruct(
    "side" / types.serum3_side.layout,
    "limit_price" / borsh.U64,
    "max_base_qty" / borsh.U64,
    "max_native_quote_qty_including_fees" / borsh.U64,
    "self_trade_behavior" / types.serum3_self_trade_behavior.layout,
    "order_type" / types.serum3_order_type.layout,
    "client_order_id" / borsh.U64,
    "limit" / borsh.U16,
)


class Serum3PlaceOrderAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    open_orders: PublicKey
    serum_market: PublicKey
    serum_program: PublicKey
    serum_market_external: PublicKey
    market_bids: PublicKey
    market_asks: PublicKey
    market_event_queue: PublicKey
    market_request_queue: PublicKey
    market_base_vault: PublicKey
    market_quote_vault: PublicKey
    market_vault_signer: PublicKey
    payer_bank: PublicKey
    payer_vault: PublicKey
    token_program: PublicKey


def serum3_place_order(
    args: Serum3PlaceOrderArgs,
    accounts: Serum3PlaceOrderAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["open_orders"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["serum_market"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_market_external"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=accounts["market_bids"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["market_asks"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["market_event_queue"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_request_queue"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_base_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_quote_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_vault_signer"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["payer_bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["payer_vault"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"a\x1d{\xc7\xe4\x14\xb8\xfc"
    encoded_args = layout.build(
        {
            "side": args["side"].to_encodable(),
            "limit_price": args["limit_price"],
            "max_base_qty": args["max_base_qty"],
            "max_native_quote_qty_including_fees": args[
                "max_native_quote_qty_including_fees"
            ],
            "self_trade_behavior": args["self_trade_behavior"].to_encodable(),
            "order_type": args["order_type"].to_encodable(),
            "client_order_id": args["client_order_id"],
            "limit": args["limit"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
