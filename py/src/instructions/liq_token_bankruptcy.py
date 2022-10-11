from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class LiqTokenBankruptcyArgs(typing.TypedDict):
    max_liab_transfer: types.i80f48.I80F48


layout = borsh.CStruct("max_liab_transfer" / types.i80f48.I80F48.layout)


class LiqTokenBankruptcyAccounts(typing.TypedDict):
    group: PublicKey
    liqor: PublicKey
    liqor_owner: PublicKey
    liqee: PublicKey
    liab_mint_info: PublicKey
    quote_vault: PublicKey
    insurance_vault: PublicKey
    token_program: PublicKey


def liq_token_bankruptcy(
    args: LiqTokenBankruptcyArgs,
    accounts: LiqTokenBankruptcyAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["liqor"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["liqor_owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["liqee"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["liab_mint_info"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["quote_vault"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["insurance_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"i\xab\xdfDk>\x0c\xf3"
    encoded_args = layout.build(
        {
            "max_liab_transfer": args["max_liab_transfer"].to_encodable(),
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
