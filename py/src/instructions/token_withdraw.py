from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class TokenWithdrawArgs(typing.TypedDict):
    amount: int
    allow_borrow: bool


layout = borsh.CStruct("amount" / borsh.U64, "allow_borrow" / borsh.Bool)


class TokenWithdrawAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey
    bank: PublicKey
    vault: PublicKey
    oracle: PublicKey
    token_account: PublicKey
    token_program: PublicKey


def token_withdraw(
    args: TokenWithdrawArgs,
    accounts: TokenWithdrawAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["vault"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=False),
        AccountMeta(
            pubkey=accounts["token_account"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"?\xdf*;\x0f\x80fB"
    encoded_args = layout.build(
        {
            "amount": args["amount"],
            "allow_borrow": args["allow_borrow"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
