from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class Serum3CancelOrderArgs(typing.TypedDict):
    side: types.serum3_side.Serum3SideKind
    order_id: int


layout = borsh.CStruct("side" / types.serum3_side.layout, "order_id" / borsh.U128)


class Serum3CancelOrderAccounts(typing.TypedDict):
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


def serum3_cancel_order(
    args: Serum3CancelOrderArgs,
    accounts: Serum3CancelOrderAccounts,
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
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"~rm\x16\xc6\x0c|K"
    encoded_args = layout.build(
        {
            "side": args["side"].to_encodable(),
            "order_id": args["order_id"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
