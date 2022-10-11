from __future__ import annotations
from . import (
    i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class PerpEquityJSON(typing.TypedDict):
    perp_market_index: int
    value: i80f48.I80F48JSON


@dataclass
class PerpEquity:
    layout: typing.ClassVar = borsh.CStruct(
        "perp_market_index" / borsh.U16, "value" / i80f48.I80F48.layout
    )
    perp_market_index: int
    value: i80f48.I80F48

    @classmethod
    def from_decoded(cls, obj: Container) -> "PerpEquity":
        return cls(
            perp_market_index=obj.perp_market_index,
            value=i80f48.I80F48.from_decoded(obj.value),
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "perp_market_index": self.perp_market_index,
            "value": self.value.to_encodable(),
        }

    def to_json(self) -> PerpEquityJSON:
        return {
            "perp_market_index": self.perp_market_index,
            "value": self.value.to_json(),
        }

    @classmethod
    def from_json(cls, obj: PerpEquityJSON) -> "PerpEquity":
        return cls(
            perp_market_index=obj["perp_market_index"],
            value=i80f48.I80F48.from_json(obj["value"]),
        )
