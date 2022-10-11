from __future__ import annotations
from . import (
    i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class TokenInfoJSON(typing.TypedDict):
    token_index: int
    maint_asset_weight: i80f48.I80F48JSON
    init_asset_weight: i80f48.I80F48JSON
    maint_liab_weight: i80f48.I80F48JSON
    init_liab_weight: i80f48.I80F48JSON
    oracle_price: i80f48.I80F48JSON
    balance: i80f48.I80F48JSON
    serum3_max_reserved: i80f48.I80F48JSON


@dataclass
class TokenInfo:
    layout: typing.ClassVar = borsh.CStruct(
        "token_index" / borsh.U16,
        "maint_asset_weight" / i80f48.I80F48.layout,
        "init_asset_weight" / i80f48.I80F48.layout,
        "maint_liab_weight" / i80f48.I80F48.layout,
        "init_liab_weight" / i80f48.I80F48.layout,
        "oracle_price" / i80f48.I80F48.layout,
        "balance" / i80f48.I80F48.layout,
        "serum3_max_reserved" / i80f48.I80F48.layout,
    )
    token_index: int
    maint_asset_weight: i80f48.I80F48
    init_asset_weight: i80f48.I80F48
    maint_liab_weight: i80f48.I80F48
    init_liab_weight: i80f48.I80F48
    oracle_price: i80f48.I80F48
    balance: i80f48.I80F48
    serum3_max_reserved: i80f48.I80F48

    @classmethod
    def from_decoded(cls, obj: Container) -> "TokenInfo":
        return cls(
            token_index=obj.token_index,
            maint_asset_weight=i80f48.I80F48.from_decoded(obj.maint_asset_weight),
            init_asset_weight=i80f48.I80F48.from_decoded(obj.init_asset_weight),
            maint_liab_weight=i80f48.I80F48.from_decoded(obj.maint_liab_weight),
            init_liab_weight=i80f48.I80F48.from_decoded(obj.init_liab_weight),
            oracle_price=i80f48.I80F48.from_decoded(obj.oracle_price),
            balance=i80f48.I80F48.from_decoded(obj.balance),
            serum3_max_reserved=i80f48.I80F48.from_decoded(obj.serum3_max_reserved),
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "token_index": self.token_index,
            "maint_asset_weight": self.maint_asset_weight.to_encodable(),
            "init_asset_weight": self.init_asset_weight.to_encodable(),
            "maint_liab_weight": self.maint_liab_weight.to_encodable(),
            "init_liab_weight": self.init_liab_weight.to_encodable(),
            "oracle_price": self.oracle_price.to_encodable(),
            "balance": self.balance.to_encodable(),
            "serum3_max_reserved": self.serum3_max_reserved.to_encodable(),
        }

    def to_json(self) -> TokenInfoJSON:
        return {
            "token_index": self.token_index,
            "maint_asset_weight": self.maint_asset_weight.to_json(),
            "init_asset_weight": self.init_asset_weight.to_json(),
            "maint_liab_weight": self.maint_liab_weight.to_json(),
            "init_liab_weight": self.init_liab_weight.to_json(),
            "oracle_price": self.oracle_price.to_json(),
            "balance": self.balance.to_json(),
            "serum3_max_reserved": self.serum3_max_reserved.to_json(),
        }

    @classmethod
    def from_json(cls, obj: TokenInfoJSON) -> "TokenInfo":
        return cls(
            token_index=obj["token_index"],
            maint_asset_weight=i80f48.I80F48.from_json(obj["maint_asset_weight"]),
            init_asset_weight=i80f48.I80F48.from_json(obj["init_asset_weight"]),
            maint_liab_weight=i80f48.I80F48.from_json(obj["maint_liab_weight"]),
            init_liab_weight=i80f48.I80F48.from_json(obj["init_liab_weight"]),
            oracle_price=i80f48.I80F48.from_json(obj["oracle_price"]),
            balance=i80f48.I80F48.from_json(obj["balance"]),
            serum3_max_reserved=i80f48.I80F48.from_json(obj["serum3_max_reserved"]),
        )
