from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class PerpLiqBasePositionArgs(typing.TypedDict):
    max_base_transfer: int


layout = borsh.CStruct("max_base_transfer" / borsh.I64)


class PerpLiqBasePositionAccounts(typing.TypedDict):
    group: PublicKey
    perp_market: PublicKey
    oracle: PublicKey
    liqor: PublicKey
    liqor_owner: PublicKey
    liqee: PublicKey


def perp_liq_base_position(
    args: PerpLiqBasePositionArgs,
    accounts: PerpLiqBasePositionAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["perp_market"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["liqor"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["liqor_owner"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["liqee"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xa8L\xc9uH5Ak"
    encoded_args = layout.build(
        {
            "max_base_transfer": args["max_base_transfer"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
