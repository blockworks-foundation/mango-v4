from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class TokenRegisterTrustlessArgs(typing.TypedDict):
    token_index: int
    name: str


layout = borsh.CStruct("token_index" / borsh.U16, "name" / borsh.String)


class TokenRegisterTrustlessAccounts(typing.TypedDict):
    group: PublicKey
    fast_listing_admin: PublicKey
    mint: PublicKey
    bank: PublicKey
    vault: PublicKey
    mint_info: PublicKey
    oracle: PublicKey
    payer: PublicKey
    token_program: PublicKey
    system_program: PublicKey
    rent: PublicKey


def token_register_trustless(
    args: TokenRegisterTrustlessArgs,
    accounts: TokenRegisterTrustlessAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(
            pubkey=accounts["fast_listing_admin"], is_signer=True, is_writable=False
        ),
        AccountMeta(pubkey=accounts["mint"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["vault"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["mint_info"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=False),
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
    identifier = b"8-#\r\xfd\xfe:P"
    encoded_args = layout.build(
        {
            "token_index": args["token_index"],
            "name": args["name"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
