import typing
from dataclasses import dataclass
from base64 import b64decode
from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Commitment
import borsh_construct as borsh
from anchorpy.coder.accounts import ACCOUNT_DISCRIMINATOR_SIZE
from anchorpy.error import AccountInvalidDiscriminator
from anchorpy.utils.rpc import get_multiple_accounts
from anchorpy.borsh_extension import BorshPubkey
from ..program_id import PROGRAM_ID
from .. import types


class PerpMarketJSON(typing.TypedDict):
    group: str
    settle_token_index: int
    perp_market_index: int
    trusted_market: int
    group_insurance_fund: int
    padding1: list[int]
    name: list[int]
    oracle: str
    oracle_config: types.oracle_config.OracleConfigJSON
    bids: str
    asks: str
    event_queue: str
    quote_lot_size: int
    base_lot_size: int
    maint_asset_weight: types.i80f48.I80F48JSON
    init_asset_weight: types.i80f48.I80F48JSON
    maint_liab_weight: types.i80f48.I80F48JSON
    init_liab_weight: types.i80f48.I80F48JSON
    liquidation_fee: types.i80f48.I80F48JSON
    maker_fee: types.i80f48.I80F48JSON
    taker_fee: types.i80f48.I80F48JSON
    min_funding: types.i80f48.I80F48JSON
    max_funding: types.i80f48.I80F48JSON
    impact_quantity: int
    long_funding: types.i80f48.I80F48JSON
    short_funding: types.i80f48.I80F48JSON
    funding_last_updated: int
    open_interest: int
    seq_num: int
    fees_accrued: types.i80f48.I80F48JSON
    bump: int
    base_decimals: int
    padding2: list[int]
    registration_time: int
    fees_settled: types.i80f48.I80F48JSON
    fee_penalty: float
    settle_fee_flat: float
    settle_fee_amount_threshold: float
    settle_fee_fraction_low_health: float
    reserved: list[int]


@dataclass
class PerpMarket:
    discriminator: typing.ClassVar = b"\n\xdf\x0c,k\xf57\xf7"
    layout: typing.ClassVar = borsh.CStruct(
        "group" / BorshPubkey,
        "settle_token_index" / borsh.U16,
        "perp_market_index" / borsh.U16,
        "trusted_market" / borsh.U8,
        "group_insurance_fund" / borsh.U8,
        "padding1" / borsh.U8[2],
        "name" / borsh.U8[16],
        "oracle" / BorshPubkey,
        "oracle_config" / types.oracle_config.OracleConfig.layout,
        "bids" / BorshPubkey,
        "asks" / BorshPubkey,
        "event_queue" / BorshPubkey,
        "quote_lot_size" / borsh.I64,
        "base_lot_size" / borsh.I64,
        "maint_asset_weight" / types.i80f48.I80F48.layout,
        "init_asset_weight" / types.i80f48.I80F48.layout,
        "maint_liab_weight" / types.i80f48.I80F48.layout,
        "init_liab_weight" / types.i80f48.I80F48.layout,
        "liquidation_fee" / types.i80f48.I80F48.layout,
        "maker_fee" / types.i80f48.I80F48.layout,
        "taker_fee" / types.i80f48.I80F48.layout,
        "min_funding" / types.i80f48.I80F48.layout,
        "max_funding" / types.i80f48.I80F48.layout,
        "impact_quantity" / borsh.I64,
        "long_funding" / types.i80f48.I80F48.layout,
        "short_funding" / types.i80f48.I80F48.layout,
        "funding_last_updated" / borsh.I64,
        "open_interest" / borsh.I64,
        "seq_num" / borsh.U64,
        "fees_accrued" / types.i80f48.I80F48.layout,
        "bump" / borsh.U8,
        "base_decimals" / borsh.U8,
        "padding2" / borsh.U8[6],
        "registration_time" / borsh.I64,
        "fees_settled" / types.i80f48.I80F48.layout,
        "fee_penalty" / borsh.F32,
        "settle_fee_flat" / borsh.F32,
        "settle_fee_amount_threshold" / borsh.F32,
        "settle_fee_fraction_low_health" / borsh.F32,
        "reserved" / borsh.U8[92],
    )
    group: PublicKey
    settle_token_index: int
    perp_market_index: int
    trusted_market: int
    group_insurance_fund: int
    padding1: list[int]
    name: list[int]
    oracle: PublicKey
    oracle_config: types.oracle_config.OracleConfig
    bids: PublicKey
    asks: PublicKey
    event_queue: PublicKey
    quote_lot_size: int
    base_lot_size: int
    maint_asset_weight: types.i80f48.I80F48
    init_asset_weight: types.i80f48.I80F48
    maint_liab_weight: types.i80f48.I80F48
    init_liab_weight: types.i80f48.I80F48
    liquidation_fee: types.i80f48.I80F48
    maker_fee: types.i80f48.I80F48
    taker_fee: types.i80f48.I80F48
    min_funding: types.i80f48.I80F48
    max_funding: types.i80f48.I80F48
    impact_quantity: int
    long_funding: types.i80f48.I80F48
    short_funding: types.i80f48.I80F48
    funding_last_updated: int
    open_interest: int
    seq_num: int
    fees_accrued: types.i80f48.I80F48
    bump: int
    base_decimals: int
    padding2: list[int]
    registration_time: int
    fees_settled: types.i80f48.I80F48
    fee_penalty: float
    settle_fee_flat: float
    settle_fee_amount_threshold: float
    settle_fee_fraction_low_health: float
    reserved: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: PublicKey,
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.Optional["PerpMarket"]:
        resp = await conn.get_account_info(address, commitment=commitment)
        info = resp["result"]["value"]
        if info is None:
            return None
        if info["owner"] != str(program_id):
            raise ValueError("Account does not belong to this program")
        bytes_data = b64decode(info["data"][0])
        return cls.decode(bytes_data)

    @classmethod
    async def fetch_multiple(
        cls,
        conn: AsyncClient,
        addresses: list[PublicKey],
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.List[typing.Optional["PerpMarket"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["PerpMarket"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "PerpMarket":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = PerpMarket.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            group=dec.group,
            settle_token_index=dec.settle_token_index,
            perp_market_index=dec.perp_market_index,
            trusted_market=dec.trusted_market,
            group_insurance_fund=dec.group_insurance_fund,
            padding1=dec.padding1,
            name=dec.name,
            oracle=dec.oracle,
            oracle_config=types.oracle_config.OracleConfig.from_decoded(
                dec.oracle_config
            ),
            bids=dec.bids,
            asks=dec.asks,
            event_queue=dec.event_queue,
            quote_lot_size=dec.quote_lot_size,
            base_lot_size=dec.base_lot_size,
            maint_asset_weight=types.i80f48.I80F48.from_decoded(dec.maint_asset_weight),
            init_asset_weight=types.i80f48.I80F48.from_decoded(dec.init_asset_weight),
            maint_liab_weight=types.i80f48.I80F48.from_decoded(dec.maint_liab_weight),
            init_liab_weight=types.i80f48.I80F48.from_decoded(dec.init_liab_weight),
            liquidation_fee=types.i80f48.I80F48.from_decoded(dec.liquidation_fee),
            maker_fee=types.i80f48.I80F48.from_decoded(dec.maker_fee),
            taker_fee=types.i80f48.I80F48.from_decoded(dec.taker_fee),
            min_funding=types.i80f48.I80F48.from_decoded(dec.min_funding),
            max_funding=types.i80f48.I80F48.from_decoded(dec.max_funding),
            impact_quantity=dec.impact_quantity,
            long_funding=types.i80f48.I80F48.from_decoded(dec.long_funding),
            short_funding=types.i80f48.I80F48.from_decoded(dec.short_funding),
            funding_last_updated=dec.funding_last_updated,
            open_interest=dec.open_interest,
            seq_num=dec.seq_num,
            fees_accrued=types.i80f48.I80F48.from_decoded(dec.fees_accrued),
            bump=dec.bump,
            base_decimals=dec.base_decimals,
            padding2=dec.padding2,
            registration_time=dec.registration_time,
            fees_settled=types.i80f48.I80F48.from_decoded(dec.fees_settled),
            fee_penalty=dec.fee_penalty,
            settle_fee_flat=dec.settle_fee_flat,
            settle_fee_amount_threshold=dec.settle_fee_amount_threshold,
            settle_fee_fraction_low_health=dec.settle_fee_fraction_low_health,
            reserved=dec.reserved,
        )

    def to_json(self) -> PerpMarketJSON:
        return {
            "group": str(self.group),
            "settle_token_index": self.settle_token_index,
            "perp_market_index": self.perp_market_index,
            "trusted_market": self.trusted_market,
            "group_insurance_fund": self.group_insurance_fund,
            "padding1": self.padding1,
            "name": self.name,
            "oracle": str(self.oracle),
            "oracle_config": self.oracle_config.to_json(),
            "bids": str(self.bids),
            "asks": str(self.asks),
            "event_queue": str(self.event_queue),
            "quote_lot_size": self.quote_lot_size,
            "base_lot_size": self.base_lot_size,
            "maint_asset_weight": self.maint_asset_weight.to_json(),
            "init_asset_weight": self.init_asset_weight.to_json(),
            "maint_liab_weight": self.maint_liab_weight.to_json(),
            "init_liab_weight": self.init_liab_weight.to_json(),
            "liquidation_fee": self.liquidation_fee.to_json(),
            "maker_fee": self.maker_fee.to_json(),
            "taker_fee": self.taker_fee.to_json(),
            "min_funding": self.min_funding.to_json(),
            "max_funding": self.max_funding.to_json(),
            "impact_quantity": self.impact_quantity,
            "long_funding": self.long_funding.to_json(),
            "short_funding": self.short_funding.to_json(),
            "funding_last_updated": self.funding_last_updated,
            "open_interest": self.open_interest,
            "seq_num": self.seq_num,
            "fees_accrued": self.fees_accrued.to_json(),
            "bump": self.bump,
            "base_decimals": self.base_decimals,
            "padding2": self.padding2,
            "registration_time": self.registration_time,
            "fees_settled": self.fees_settled.to_json(),
            "fee_penalty": self.fee_penalty,
            "settle_fee_flat": self.settle_fee_flat,
            "settle_fee_amount_threshold": self.settle_fee_amount_threshold,
            "settle_fee_fraction_low_health": self.settle_fee_fraction_low_health,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: PerpMarketJSON) -> "PerpMarket":
        return cls(
            group=PublicKey(obj["group"]),
            settle_token_index=obj["settle_token_index"],
            perp_market_index=obj["perp_market_index"],
            trusted_market=obj["trusted_market"],
            group_insurance_fund=obj["group_insurance_fund"],
            padding1=obj["padding1"],
            name=obj["name"],
            oracle=PublicKey(obj["oracle"]),
            oracle_config=types.oracle_config.OracleConfig.from_json(
                obj["oracle_config"]
            ),
            bids=PublicKey(obj["bids"]),
            asks=PublicKey(obj["asks"]),
            event_queue=PublicKey(obj["event_queue"]),
            quote_lot_size=obj["quote_lot_size"],
            base_lot_size=obj["base_lot_size"],
            maint_asset_weight=types.i80f48.I80F48.from_json(obj["maint_asset_weight"]),
            init_asset_weight=types.i80f48.I80F48.from_json(obj["init_asset_weight"]),
            maint_liab_weight=types.i80f48.I80F48.from_json(obj["maint_liab_weight"]),
            init_liab_weight=types.i80f48.I80F48.from_json(obj["init_liab_weight"]),
            liquidation_fee=types.i80f48.I80F48.from_json(obj["liquidation_fee"]),
            maker_fee=types.i80f48.I80F48.from_json(obj["maker_fee"]),
            taker_fee=types.i80f48.I80F48.from_json(obj["taker_fee"]),
            min_funding=types.i80f48.I80F48.from_json(obj["min_funding"]),
            max_funding=types.i80f48.I80F48.from_json(obj["max_funding"]),
            impact_quantity=obj["impact_quantity"],
            long_funding=types.i80f48.I80F48.from_json(obj["long_funding"]),
            short_funding=types.i80f48.I80F48.from_json(obj["short_funding"]),
            funding_last_updated=obj["funding_last_updated"],
            open_interest=obj["open_interest"],
            seq_num=obj["seq_num"],
            fees_accrued=types.i80f48.I80F48.from_json(obj["fees_accrued"]),
            bump=obj["bump"],
            base_decimals=obj["base_decimals"],
            padding2=obj["padding2"],
            registration_time=obj["registration_time"],
            fees_settled=types.i80f48.I80F48.from_json(obj["fees_settled"]),
            fee_penalty=obj["fee_penalty"],
            settle_fee_flat=obj["settle_fee_flat"],
            settle_fee_amount_threshold=obj["settle_fee_amount_threshold"],
            settle_fee_fraction_low_health=obj["settle_fee_fraction_low_health"],
            reserved=obj["reserved"],
        )
