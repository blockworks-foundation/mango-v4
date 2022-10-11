from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class PerpEditMarketArgs(typing.TypedDict):
    oracle_opt: typing.Optional[PublicKey]
    oracle_config_opt: typing.Optional[types.oracle_config.OracleConfig]
    base_decimals_opt: typing.Optional[int]
    maint_asset_weight_opt: typing.Optional[float]
    init_asset_weight_opt: typing.Optional[float]
    maint_liab_weight_opt: typing.Optional[float]
    init_liab_weight_opt: typing.Optional[float]
    liquidation_fee_opt: typing.Optional[float]
    maker_fee_opt: typing.Optional[float]
    taker_fee_opt: typing.Optional[float]
    min_funding_opt: typing.Optional[float]
    max_funding_opt: typing.Optional[float]
    impact_quantity_opt: typing.Optional[int]
    group_insurance_fund_opt: typing.Optional[bool]
    trusted_market_opt: typing.Optional[bool]
    fee_penalty_opt: typing.Optional[float]
    settle_fee_flat_opt: typing.Optional[float]
    settle_fee_amount_threshold_opt: typing.Optional[float]
    settle_fee_fraction_low_health_opt: typing.Optional[float]


layout = borsh.CStruct(
    "oracle_opt" / borsh.Option(BorshPubkey),
    "oracle_config_opt" / borsh.Option(types.oracle_config.OracleConfig.layout),
    "base_decimals_opt" / borsh.Option(borsh.U8),
    "maint_asset_weight_opt" / borsh.Option(borsh.F32),
    "init_asset_weight_opt" / borsh.Option(borsh.F32),
    "maint_liab_weight_opt" / borsh.Option(borsh.F32),
    "init_liab_weight_opt" / borsh.Option(borsh.F32),
    "liquidation_fee_opt" / borsh.Option(borsh.F32),
    "maker_fee_opt" / borsh.Option(borsh.F32),
    "taker_fee_opt" / borsh.Option(borsh.F32),
    "min_funding_opt" / borsh.Option(borsh.F32),
    "max_funding_opt" / borsh.Option(borsh.F32),
    "impact_quantity_opt" / borsh.Option(borsh.I64),
    "group_insurance_fund_opt" / borsh.Option(borsh.Bool),
    "trusted_market_opt" / borsh.Option(borsh.Bool),
    "fee_penalty_opt" / borsh.Option(borsh.F32),
    "settle_fee_flat_opt" / borsh.Option(borsh.F32),
    "settle_fee_amount_threshold_opt" / borsh.Option(borsh.F32),
    "settle_fee_fraction_low_health_opt" / borsh.Option(borsh.F32),
)


class PerpEditMarketAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey
    perp_market: PublicKey


def perp_edit_market(
    args: PerpEditMarketArgs,
    accounts: PerpEditMarketAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["perp_market"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"|r\xa0\xe7E\xdfLQ"
    encoded_args = layout.build(
        {
            "oracle_opt": args["oracle_opt"],
            "oracle_config_opt": (
                None
                if args["oracle_config_opt"] is None
                else args["oracle_config_opt"].to_encodable()
            ),
            "base_decimals_opt": args["base_decimals_opt"],
            "maint_asset_weight_opt": args["maint_asset_weight_opt"],
            "init_asset_weight_opt": args["init_asset_weight_opt"],
            "maint_liab_weight_opt": args["maint_liab_weight_opt"],
            "init_liab_weight_opt": args["init_liab_weight_opt"],
            "liquidation_fee_opt": args["liquidation_fee_opt"],
            "maker_fee_opt": args["maker_fee_opt"],
            "taker_fee_opt": args["taker_fee_opt"],
            "min_funding_opt": args["min_funding_opt"],
            "max_funding_opt": args["max_funding_opt"],
            "impact_quantity_opt": args["impact_quantity_opt"],
            "group_insurance_fund_opt": args["group_insurance_fund_opt"],
            "trusted_market_opt": args["trusted_market_opt"],
            "fee_penalty_opt": args["fee_penalty_opt"],
            "settle_fee_flat_opt": args["settle_fee_flat_opt"],
            "settle_fee_amount_threshold_opt": args["settle_fee_amount_threshold_opt"],
            "settle_fee_fraction_low_health_opt": args[
                "settle_fee_fraction_low_health_opt"
            ],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
