from __future__ import annotations
from . import (
    side,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class PerpOpenOrderJSON(typing.TypedDict):
    order_side: side.SideJSON
    padding1: list[int]
    order_market: int
    padding2: list[int]
    client_order_id: int
    order_id: int
    reserved: list[int]


@dataclass
class PerpOpenOrder:
    layout: typing.ClassVar = borsh.CStruct(
        "order_side" / side.layout,
        "padding1" / borsh.U8[1],
        "order_market" / borsh.U16,
        "padding2" / borsh.U8[4],
        "client_order_id" / borsh.U64,
        "order_id" / borsh.I128,
        "reserved" / borsh.U8[64],
    )
    order_side: side.SideKind
    padding1: list[int]
    order_market: int
    padding2: list[int]
    client_order_id: int
    order_id: int
    reserved: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "PerpOpenOrder":
        return cls(
            order_side=side.from_decoded(obj.order_side),
            padding1=obj.padding1,
            order_market=obj.order_market,
            padding2=obj.padding2,
            client_order_id=obj.client_order_id,
            order_id=obj.order_id,
            reserved=obj.reserved,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "order_side": self.order_side.to_encodable(),
            "padding1": self.padding1,
            "order_market": self.order_market,
            "padding2": self.padding2,
            "client_order_id": self.client_order_id,
            "order_id": self.order_id,
            "reserved": self.reserved,
        }

    def to_json(self) -> PerpOpenOrderJSON:
        return {
            "order_side": self.order_side.to_json(),
            "padding1": self.padding1,
            "order_market": self.order_market,
            "padding2": self.padding2,
            "client_order_id": self.client_order_id,
            "order_id": self.order_id,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: PerpOpenOrderJSON) -> "PerpOpenOrder":
        return cls(
            order_side=side.from_json(obj["order_side"]),
            padding1=obj["padding1"],
            order_market=obj["order_market"],
            padding2=obj["padding2"],
            client_order_id=obj["client_order_id"],
            order_id=obj["order_id"],
            reserved=obj["reserved"],
        )
