from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class PerpLiqBankruptcyArgs(typing.TypedDict):
    max_liab_transfer: int


layout = borsh.CStruct("max_liab_transfer" / borsh.U64)


class PerpLiqBankruptcyAccounts(typing.TypedDict):
    group: PublicKey
    perp_market: PublicKey
    liqor: PublicKey
    liqor_owner: PublicKey
    liqee: PublicKey
    settle_bank: PublicKey
    settle_vault: PublicKey
    settle_oracle: PublicKey
    insurance_vault: PublicKey
    token_program: PublicKey


def perp_liq_bankruptcy(
    args: PerpLiqBankruptcyArgs,
    accounts: PerpLiqBankruptcyAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["perp_market"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["liqor"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["liqor_owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["liqee"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["settle_bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["settle_vault"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["settle_oracle"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["insurance_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\x8b\xcc\xdd\x02\xe9\x05\xb5|"
    encoded_args = layout.build(
        {
            "max_liab_transfer": args["max_liab_transfer"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
