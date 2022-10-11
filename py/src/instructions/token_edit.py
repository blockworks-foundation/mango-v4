from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class TokenEditArgs(typing.TypedDict):
    oracle_opt: typing.Optional[PublicKey]
    oracle_config_opt: typing.Optional[types.oracle_config.OracleConfig]
    group_insurance_fund_opt: typing.Optional[bool]
    interest_rate_params_opt: typing.Optional[
        types.interest_rate_params.InterestRateParams
    ]
    loan_fee_rate_opt: typing.Optional[float]
    loan_origination_fee_rate_opt: typing.Optional[float]
    maint_asset_weight_opt: typing.Optional[float]
    init_asset_weight_opt: typing.Optional[float]
    maint_liab_weight_opt: typing.Optional[float]
    init_liab_weight_opt: typing.Optional[float]
    liquidation_fee_opt: typing.Optional[float]


layout = borsh.CStruct(
    "oracle_opt" / borsh.Option(BorshPubkey),
    "oracle_config_opt" / borsh.Option(types.oracle_config.OracleConfig.layout),
    "group_insurance_fund_opt" / borsh.Option(borsh.Bool),
    "interest_rate_params_opt"
    / borsh.Option(types.interest_rate_params.InterestRateParams.layout),
    "loan_fee_rate_opt" / borsh.Option(borsh.F32),
    "loan_origination_fee_rate_opt" / borsh.Option(borsh.F32),
    "maint_asset_weight_opt" / borsh.Option(borsh.F32),
    "init_asset_weight_opt" / borsh.Option(borsh.F32),
    "maint_liab_weight_opt" / borsh.Option(borsh.F32),
    "init_liab_weight_opt" / borsh.Option(borsh.F32),
    "liquidation_fee_opt" / borsh.Option(borsh.F32),
)


class TokenEditAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey
    mint_info: PublicKey


def token_edit(
    args: TokenEditArgs,
    accounts: TokenEditAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["mint_info"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\x91\xcc\x0b\xd1\xae\x86O>"
    encoded_args = layout.build(
        {
            "oracle_opt": args["oracle_opt"],
            "oracle_config_opt": (
                None
                if args["oracle_config_opt"] is None
                else args["oracle_config_opt"].to_encodable()
            ),
            "group_insurance_fund_opt": args["group_insurance_fund_opt"],
            "interest_rate_params_opt": (
                None
                if args["interest_rate_params_opt"] is None
                else args["interest_rate_params_opt"].to_encodable()
            ),
            "loan_fee_rate_opt": args["loan_fee_rate_opt"],
            "loan_origination_fee_rate_opt": args["loan_origination_fee_rate_opt"],
            "maint_asset_weight_opt": args["maint_asset_weight_opt"],
            "init_asset_weight_opt": args["init_asset_weight_opt"],
            "maint_liab_weight_opt": args["maint_liab_weight_opt"],
            "init_liab_weight_opt": args["init_liab_weight_opt"],
            "liquidation_fee_opt": args["liquidation_fee_opt"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
