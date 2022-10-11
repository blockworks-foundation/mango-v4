from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class LiqTokenWithTokenArgs(typing.TypedDict):
    asset_token_index: int
    liab_token_index: int
    max_liab_transfer: types.i80f48.I80F48


layout = borsh.CStruct(
    "asset_token_index" / borsh.U16,
    "liab_token_index" / borsh.U16,
    "max_liab_transfer" / types.i80f48.I80F48.layout,
)


class LiqTokenWithTokenAccounts(typing.TypedDict):
    group: PublicKey
    liqor: PublicKey
    liqor_owner: PublicKey
    liqee: PublicKey


def liq_token_with_token(
    args: LiqTokenWithTokenArgs,
    accounts: LiqTokenWithTokenAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["liqor"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["liqor_owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["liqee"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"C\x7f\x98\x98\xd3\xd0\xfb\xe2"
    encoded_args = layout.build(
        {
            "asset_token_index": args["asset_token_index"],
            "liab_token_index": args["liab_token_index"],
            "max_liab_transfer": args["max_liab_transfer"].to_encodable(),
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
