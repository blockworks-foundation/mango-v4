from __future__ import annotations
from . import (
    i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class Serum3InfoJSON(typing.TypedDict):
    reserved: i80f48.I80F48JSON
    base_index: int
    quote_index: int
    market_index: int


@dataclass
class Serum3Info:
    layout: typing.ClassVar = borsh.CStruct(
        "reserved" / i80f48.I80F48.layout,
        "base_index" / borsh.U64,
        "quote_index" / borsh.U64,
        "market_index" / borsh.U16,
    )
    reserved: i80f48.I80F48
    base_index: int
    quote_index: int
    market_index: int

    @classmethod
    def from_decoded(cls, obj: Container) -> "Serum3Info":
        return cls(
            reserved=i80f48.I80F48.from_decoded(obj.reserved),
            base_index=obj.base_index,
            quote_index=obj.quote_index,
            market_index=obj.market_index,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "reserved": self.reserved.to_encodable(),
            "base_index": self.base_index,
            "quote_index": self.quote_index,
            "market_index": self.market_index,
        }

    def to_json(self) -> Serum3InfoJSON:
        return {
            "reserved": self.reserved.to_json(),
            "base_index": self.base_index,
            "quote_index": self.quote_index,
            "market_index": self.market_index,
        }

    @classmethod
    def from_json(cls, obj: Serum3InfoJSON) -> "Serum3Info":
        return cls(
            reserved=i80f48.I80F48.from_json(obj["reserved"]),
            base_index=obj["base_index"],
            quote_index=obj["quote_index"],
            market_index=obj["market_index"],
        )
