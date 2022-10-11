from __future__ import annotations
from . import (
    order_type,
)
import typing
from dataclasses import dataclass
from construct import Container
from solana.publickey import PublicKey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class LeafNodeJSON(typing.TypedDict):
    tag: int
    owner_slot: int
    order_type: order_type.OrderTypeJSON
    padding: list[int]
    time_in_force: int
    key: int
    owner: str
    quantity: int
    client_order_id: int
    timestamp: int
    reserved: list[int]


@dataclass
class LeafNode:
    layout: typing.ClassVar = borsh.CStruct(
        "tag" / borsh.U32,
        "owner_slot" / borsh.U8,
        "order_type" / order_type.layout,
        "padding" / borsh.U8[1],
        "time_in_force" / borsh.U8,
        "key" / borsh.I128,
        "owner" / BorshPubkey,
        "quantity" / borsh.I64,
        "client_order_id" / borsh.U64,
        "timestamp" / borsh.U64,
        "reserved" / borsh.U8[16],
    )
    tag: int
    owner_slot: int
    order_type: order_type.OrderTypeKind
    padding: list[int]
    time_in_force: int
    key: int
    owner: PublicKey
    quantity: int
    client_order_id: int
    timestamp: int
    reserved: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "LeafNode":
        return cls(
            tag=obj.tag,
            owner_slot=obj.owner_slot,
            order_type=order_type.from_decoded(obj.order_type),
            padding=obj.padding,
            time_in_force=obj.time_in_force,
            key=obj.key,
            owner=obj.owner,
            quantity=obj.quantity,
            client_order_id=obj.client_order_id,
            timestamp=obj.timestamp,
            reserved=obj.reserved,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "tag": self.tag,
            "owner_slot": self.owner_slot,
            "order_type": self.order_type.to_encodable(),
            "padding": self.padding,
            "time_in_force": self.time_in_force,
            "key": self.key,
            "owner": self.owner,
            "quantity": self.quantity,
            "client_order_id": self.client_order_id,
            "timestamp": self.timestamp,
            "reserved": self.reserved,
        }

    def to_json(self) -> LeafNodeJSON:
        return {
            "tag": self.tag,
            "owner_slot": self.owner_slot,
            "order_type": self.order_type.to_json(),
            "padding": self.padding,
            "time_in_force": self.time_in_force,
            "key": self.key,
            "owner": str(self.owner),
            "quantity": self.quantity,
            "client_order_id": self.client_order_id,
            "timestamp": self.timestamp,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: LeafNodeJSON) -> "LeafNode":
        return cls(
            tag=obj["tag"],
            owner_slot=obj["owner_slot"],
            order_type=order_type.from_json(obj["order_type"]),
            padding=obj["padding"],
            time_in_force=obj["time_in_force"],
            key=obj["key"],
            owner=PublicKey(obj["owner"]),
            quantity=obj["quantity"],
            client_order_id=obj["client_order_id"],
            timestamp=obj["timestamp"],
            reserved=obj["reserved"],
        )
