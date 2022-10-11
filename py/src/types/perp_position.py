from __future__ import annotations
from . import (
    i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class PerpPositionJSON(typing.TypedDict):
    market_index: int
    padding: list[int]
    base_position_lots: int
    quote_position_native: i80f48.I80F48JSON
    quote_entry_native: int
    quote_running_native: int
    long_settled_funding: i80f48.I80F48JSON
    short_settled_funding: i80f48.I80F48JSON
    bids_base_lots: int
    asks_base_lots: int
    taker_base_lots: int
    taker_quote_lots: int
    reserved: list[int]


@dataclass
class PerpPosition:
    layout: typing.ClassVar = borsh.CStruct(
        "market_index" / borsh.U16,
        "padding" / borsh.U8[6],
        "base_position_lots" / borsh.I64,
        "quote_position_native" / i80f48.I80F48.layout,
        "quote_entry_native" / borsh.I64,
        "quote_running_native" / borsh.I64,
        "long_settled_funding" / i80f48.I80F48.layout,
        "short_settled_funding" / i80f48.I80F48.layout,
        "bids_base_lots" / borsh.I64,
        "asks_base_lots" / borsh.I64,
        "taker_base_lots" / borsh.I64,
        "taker_quote_lots" / borsh.I64,
        "reserved" / borsh.U8[64],
    )
    market_index: int
    padding: list[int]
    base_position_lots: int
    quote_position_native: i80f48.I80F48
    quote_entry_native: int
    quote_running_native: int
    long_settled_funding: i80f48.I80F48
    short_settled_funding: i80f48.I80F48
    bids_base_lots: int
    asks_base_lots: int
    taker_base_lots: int
    taker_quote_lots: int
    reserved: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "PerpPosition":
        return cls(
            market_index=obj.market_index,
            padding=obj.padding,
            base_position_lots=obj.base_position_lots,
            quote_position_native=i80f48.I80F48.from_decoded(obj.quote_position_native),
            quote_entry_native=obj.quote_entry_native,
            quote_running_native=obj.quote_running_native,
            long_settled_funding=i80f48.I80F48.from_decoded(obj.long_settled_funding),
            short_settled_funding=i80f48.I80F48.from_decoded(obj.short_settled_funding),
            bids_base_lots=obj.bids_base_lots,
            asks_base_lots=obj.asks_base_lots,
            taker_base_lots=obj.taker_base_lots,
            taker_quote_lots=obj.taker_quote_lots,
            reserved=obj.reserved,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "market_index": self.market_index,
            "padding": self.padding,
            "base_position_lots": self.base_position_lots,
            "quote_position_native": self.quote_position_native.to_encodable(),
            "quote_entry_native": self.quote_entry_native,
            "quote_running_native": self.quote_running_native,
            "long_settled_funding": self.long_settled_funding.to_encodable(),
            "short_settled_funding": self.short_settled_funding.to_encodable(),
            "bids_base_lots": self.bids_base_lots,
            "asks_base_lots": self.asks_base_lots,
            "taker_base_lots": self.taker_base_lots,
            "taker_quote_lots": self.taker_quote_lots,
            "reserved": self.reserved,
        }

    def to_json(self) -> PerpPositionJSON:
        return {
            "market_index": self.market_index,
            "padding": self.padding,
            "base_position_lots": self.base_position_lots,
            "quote_position_native": self.quote_position_native.to_json(),
            "quote_entry_native": self.quote_entry_native,
            "quote_running_native": self.quote_running_native,
            "long_settled_funding": self.long_settled_funding.to_json(),
            "short_settled_funding": self.short_settled_funding.to_json(),
            "bids_base_lots": self.bids_base_lots,
            "asks_base_lots": self.asks_base_lots,
            "taker_base_lots": self.taker_base_lots,
            "taker_quote_lots": self.taker_quote_lots,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: PerpPositionJSON) -> "PerpPosition":
        return cls(
            market_index=obj["market_index"],
            padding=obj["padding"],
            base_position_lots=obj["base_position_lots"],
            quote_position_native=i80f48.I80F48.from_json(obj["quote_position_native"]),
            quote_entry_native=obj["quote_entry_native"],
            quote_running_native=obj["quote_running_native"],
            long_settled_funding=i80f48.I80F48.from_json(obj["long_settled_funding"]),
            short_settled_funding=i80f48.I80F48.from_json(obj["short_settled_funding"]),
            bids_base_lots=obj["bids_base_lots"],
            asks_base_lots=obj["asks_base_lots"],
            taker_base_lots=obj["taker_base_lots"],
            taker_quote_lots=obj["taker_quote_lots"],
            reserved=obj["reserved"],
        )
