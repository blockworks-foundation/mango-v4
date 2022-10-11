from __future__ import annotations
from . import (
    i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class PerpInfoJSON(typing.TypedDict):
    perp_market_index: int
    maint_asset_weight: i80f48.I80F48JSON
    init_asset_weight: i80f48.I80F48JSON
    maint_liab_weight: i80f48.I80F48JSON
    init_liab_weight: i80f48.I80F48JSON
    base: i80f48.I80F48JSON
    quote: i80f48.I80F48JSON
    oracle_price: i80f48.I80F48JSON
    has_open_orders: bool
    trusted_market: bool


@dataclass
class PerpInfo:
    layout: typing.ClassVar = borsh.CStruct(
        "perp_market_index" / borsh.U16,
        "maint_asset_weight" / i80f48.I80F48.layout,
        "init_asset_weight" / i80f48.I80F48.layout,
        "maint_liab_weight" / i80f48.I80F48.layout,
        "init_liab_weight" / i80f48.I80F48.layout,
        "base" / i80f48.I80F48.layout,
        "quote" / i80f48.I80F48.layout,
        "oracle_price" / i80f48.I80F48.layout,
        "has_open_orders" / borsh.Bool,
        "trusted_market" / borsh.Bool,
    )
    perp_market_index: int
    maint_asset_weight: i80f48.I80F48
    init_asset_weight: i80f48.I80F48
    maint_liab_weight: i80f48.I80F48
    init_liab_weight: i80f48.I80F48
    base: i80f48.I80F48
    quote: i80f48.I80F48
    oracle_price: i80f48.I80F48
    has_open_orders: bool
    trusted_market: bool

    @classmethod
    def from_decoded(cls, obj: Container) -> "PerpInfo":
        return cls(
            perp_market_index=obj.perp_market_index,
            maint_asset_weight=i80f48.I80F48.from_decoded(obj.maint_asset_weight),
            init_asset_weight=i80f48.I80F48.from_decoded(obj.init_asset_weight),
            maint_liab_weight=i80f48.I80F48.from_decoded(obj.maint_liab_weight),
            init_liab_weight=i80f48.I80F48.from_decoded(obj.init_liab_weight),
            base=i80f48.I80F48.from_decoded(obj.base),
            quote=i80f48.I80F48.from_decoded(obj.quote),
            oracle_price=i80f48.I80F48.from_decoded(obj.oracle_price),
            has_open_orders=obj.has_open_orders,
            trusted_market=obj.trusted_market,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "perp_market_index": self.perp_market_index,
            "maint_asset_weight": self.maint_asset_weight.to_encodable(),
            "init_asset_weight": self.init_asset_weight.to_encodable(),
            "maint_liab_weight": self.maint_liab_weight.to_encodable(),
            "init_liab_weight": self.init_liab_weight.to_encodable(),
            "base": self.base.to_encodable(),
            "quote": self.quote.to_encodable(),
            "oracle_price": self.oracle_price.to_encodable(),
            "has_open_orders": self.has_open_orders,
            "trusted_market": self.trusted_market,
        }

    def to_json(self) -> PerpInfoJSON:
        return {
            "perp_market_index": self.perp_market_index,
            "maint_asset_weight": self.maint_asset_weight.to_json(),
            "init_asset_weight": self.init_asset_weight.to_json(),
            "maint_liab_weight": self.maint_liab_weight.to_json(),
            "init_liab_weight": self.init_liab_weight.to_json(),
            "base": self.base.to_json(),
            "quote": self.quote.to_json(),
            "oracle_price": self.oracle_price.to_json(),
            "has_open_orders": self.has_open_orders,
            "trusted_market": self.trusted_market,
        }

    @classmethod
    def from_json(cls, obj: PerpInfoJSON) -> "PerpInfo":
        return cls(
            perp_market_index=obj["perp_market_index"],
            maint_asset_weight=i80f48.I80F48.from_json(obj["maint_asset_weight"]),
            init_asset_weight=i80f48.I80F48.from_json(obj["init_asset_weight"]),
            maint_liab_weight=i80f48.I80F48.from_json(obj["maint_liab_weight"]),
            init_liab_weight=i80f48.I80F48.from_json(obj["init_liab_weight"]),
            base=i80f48.I80F48.from_json(obj["base"]),
            quote=i80f48.I80F48.from_json(obj["quote"]),
            oracle_price=i80f48.I80F48.from_json(obj["oracle_price"]),
            has_open_orders=obj["has_open_orders"],
            trusted_market=obj["trusted_market"],
        )
