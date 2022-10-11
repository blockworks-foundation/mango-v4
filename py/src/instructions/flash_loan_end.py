from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class FlashLoanEndArgs(typing.TypedDict):
    flash_loan_type: types.flash_loan_type.FlashLoanTypeKind


layout = borsh.CStruct("flash_loan_type" / types.flash_loan_type.layout)


class FlashLoanEndAccounts(typing.TypedDict):
    account: PublicKey
    owner: PublicKey
    token_program: PublicKey


def flash_loan_end(
    args: FlashLoanEndArgs,
    accounts: FlashLoanEndAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xb2\xaa\x02N\xf0\x17\xbe\xb2"
    encoded_args = layout.build(
        {
            "flash_loan_type": args["flash_loan_type"].to_encodable(),
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
