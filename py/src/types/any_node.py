from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class AnyNodeJSON(typing.TypedDict):
    tag: int
    data: list[int]


@dataclass
class AnyNode:
    layout: typing.ClassVar = borsh.CStruct("tag" / borsh.U32, "data" / borsh.U8[92])
    tag: int
    data: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "AnyNode":
        return cls(tag=obj.tag, data=obj.data)

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"tag": self.tag, "data": self.data}

    def to_json(self) -> AnyNodeJSON:
        return {"tag": self.tag, "data": self.data}

    @classmethod
    def from_json(cls, obj: AnyNodeJSON) -> "AnyNode":
        return cls(tag=obj["tag"], data=obj["data"])
