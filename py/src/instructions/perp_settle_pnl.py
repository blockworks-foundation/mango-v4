from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from ..program_id import PROGRAM_ID


class PerpSettlePnlAccounts(typing.TypedDict):
    group: PublicKey
    settler: PublicKey
    settler_owner: PublicKey
    perp_market: PublicKey
    account_a: PublicKey
    account_b: PublicKey
    oracle: PublicKey
    settle_bank: PublicKey
    settle_oracle: PublicKey


def perp_settle_pnl(
    accounts: PerpSettlePnlAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["settler"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["settler_owner"], is_signer=True, is_writable=False
        ),
        AccountMeta(pubkey=accounts["perp_market"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account_a"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["account_b"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["settle_bank"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["settle_oracle"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xf5bU\xb3\xe6\xd7\x829"
    encoded_args = b""
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
