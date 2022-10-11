from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class TokenAddBankArgs(typing.TypedDict):
    token_index: int
    bank_num: int


layout = borsh.CStruct("token_index" / borsh.U16, "bank_num" / borsh.U32)


class TokenAddBankAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey
    mint: PublicKey
    existing_bank: PublicKey
    bank: PublicKey
    vault: PublicKey
    mint_info: PublicKey
    payer: PublicKey
    token_program: PublicKey
    system_program: PublicKey
    rent: PublicKey


def token_add_bank(
    args: TokenAddBankArgs,
    accounts: TokenAddBankAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["mint"], is_signer=False, is_writable=False),
        AccountMeta(
            pubkey=accounts["existing_bank"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["vault"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["mint_info"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["payer"], is_signer=True, is_writable=True),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["system_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["rent"], is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xa3X\xea\x1f\x81\xde\x03$"
    encoded_args = layout.build(
        {
            "token_index": args["token_index"],
            "bank_num": args["bank_num"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
