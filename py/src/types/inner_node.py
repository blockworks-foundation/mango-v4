from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class InnerNodeJSON(typing.TypedDict):
    tag: int
    prefix_len: int
    key: int
    children: list[int]
    child_earliest_expiry: list[int]
    reserved: list[int]


@dataclass
class InnerNode:
    layout: typing.ClassVar = borsh.CStruct(
        "tag" / borsh.U32,
        "prefix_len" / borsh.U32,
        "key" / borsh.I128,
        "children" / borsh.U32[2],
        "child_earliest_expiry" / borsh.U64[2],
        "reserved" / borsh.U8[48],
    )
    tag: int
    prefix_len: int
    key: int
    children: list[int]
    child_earliest_expiry: list[int]
    reserved: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "InnerNode":
        return cls(
            tag=obj.tag,
            prefix_len=obj.prefix_len,
            key=obj.key,
            children=obj.children,
            child_earliest_expiry=obj.child_earliest_expiry,
            reserved=obj.reserved,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "tag": self.tag,
            "prefix_len": self.prefix_len,
            "key": self.key,
            "children": self.children,
            "child_earliest_expiry": self.child_earliest_expiry,
            "reserved": self.reserved,
        }

    def to_json(self) -> InnerNodeJSON:
        return {
            "tag": self.tag,
            "prefix_len": self.prefix_len,
            "key": self.key,
            "children": self.children,
            "child_earliest_expiry": self.child_earliest_expiry,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: InnerNodeJSON) -> "InnerNode":
        return cls(
            tag=obj["tag"],
            prefix_len=obj["prefix_len"],
            key=obj["key"],
            children=obj["children"],
            child_earliest_expiry=obj["child_earliest_expiry"],
            reserved=obj["reserved"],
        )
