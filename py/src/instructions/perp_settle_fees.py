from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class PerpSettleFeesArgs(typing.TypedDict):
    max_settle_amount: int


layout = borsh.CStruct("max_settle_amount" / borsh.U64)


class PerpSettleFeesAccounts(typing.TypedDict):
    group: PublicKey
    perp_market: PublicKey
    account: PublicKey
    oracle: PublicKey
    settle_bank: PublicKey
    settle_oracle: PublicKey


def perp_settle_fees(
    args: PerpSettleFeesArgs,
    accounts: PerpSettleFeesAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["perp_market"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["settle_bank"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["settle_oracle"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xdf\xed\xe3H\x98\xb9\xeas"
    encoded_args = layout.build(
        {
            "max_settle_amount": args["max_settle_amount"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
