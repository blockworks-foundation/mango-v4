from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class PerpCancelOrderByClientOrderIdArgs(typing.TypedDict):
    client_order_id: int


layout = borsh.CStruct("client_order_id" / borsh.U64)


class PerpCancelOrderByClientOrderIdAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    perp_market: PublicKey
    asks: PublicKey
    bids: PublicKey


def perp_cancel_order_by_client_order_id(
    args: PerpCancelOrderByClientOrderIdArgs,
    accounts: PerpCancelOrderByClientOrderIdAccounts,
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
    identifier = b"J\xfa8O\xce\xad\xa3f"
    encoded_args = layout.build(
        {
            "client_order_id": args["client_order_id"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
