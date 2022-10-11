from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from construct import Construct
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class FlashLoanBeginArgs(typing.TypedDict):
    loan_amounts: list[int]


layout = borsh.CStruct("loan_amounts" / borsh.Vec(typing.cast(Construct, borsh.U64)))


class FlashLoanBeginAccounts(typing.TypedDict):
    account: PublicKey
    owner: PublicKey
    token_program: PublicKey
    instructions: PublicKey


def flash_loan_begin(
    args: FlashLoanBeginArgs,
    accounts: FlashLoanBeginAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["instructions"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"QN\xe0<\xf48Z\xef"
    encoded_args = layout.build(
        {
            "loan_amounts": args["loan_amounts"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
