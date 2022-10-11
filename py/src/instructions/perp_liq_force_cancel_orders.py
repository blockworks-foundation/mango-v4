from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class PerpLiqForceCancelOrdersArgs(typing.TypedDict):
    limit: int


layout = borsh.CStruct("limit" / borsh.U8)


class PerpLiqForceCancelOrdersAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    perp_market: PublicKey
    asks: PublicKey
    bids: PublicKey
    oracle: PublicKey


def perp_liq_force_cancel_orders(
    args: PerpLiqForceCancelOrdersArgs,
    accounts: PerpLiqForceCancelOrdersAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["perp_market"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["asks"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["bids"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"m\xcb\xba\x10\xe9[\x01\x8d"
    encoded_args = layout.build(
        {
            "limit": args["limit"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
