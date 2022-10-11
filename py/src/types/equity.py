from __future__ import annotations
from . import (
    token_equity,
    perp_equity,
)
import typing
from dataclasses import dataclass
from construct import Container, Construct
import borsh_construct as borsh


class EquityJSON(typing.TypedDict):
    tokens: list[token_equity.TokenEquityJSON]
    perps: list[perp_equity.PerpEquityJSON]


@dataclass
class Equity:
    layout: typing.ClassVar = borsh.CStruct(
        "tokens" / borsh.Vec(typing.cast(Construct, token_equity.TokenEquity.layout)),
        "perps" / borsh.Vec(typing.cast(Construct, perp_equity.PerpEquity.layout)),
    )
    tokens: list[token_equity.TokenEquity]
    perps: list[perp_equity.PerpEquity]

    @classmethod
    def from_decoded(cls, obj: Container) -> "Equity":
        return cls(
            tokens=list(
                map(
                    lambda item: token_equity.TokenEquity.from_decoded(item), obj.tokens
                )
            ),
            perps=list(
                map(lambda item: perp_equity.PerpEquity.from_decoded(item), obj.perps)
            ),
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "tokens": list(map(lambda item: item.to_encodable(), self.tokens)),
            "perps": list(map(lambda item: item.to_encodable(), self.perps)),
        }

    def to_json(self) -> EquityJSON:
        return {
            "tokens": list(map(lambda item: item.to_json(), self.tokens)),
            "perps": list(map(lambda item: item.to_json(), self.perps)),
        }

    @classmethod
    def from_json(cls, obj: EquityJSON) -> "Equity":
        return cls(
            tokens=list(
                map(
                    lambda item: token_equity.TokenEquity.from_json(item), obj["tokens"]
                )
            ),
            perps=list(
                map(lambda item: perp_equity.PerpEquity.from_json(item), obj["perps"])
            ),
        )
