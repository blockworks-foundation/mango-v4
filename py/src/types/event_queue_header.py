from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class EventQueueHeaderJSON(typing.TypedDict):
    head: int
    count: int
    seq_num: int


@dataclass
class EventQueueHeader:
    layout: typing.ClassVar = borsh.CStruct(
        "head" / borsh.U32, "count" / borsh.U32, "seq_num" / borsh.U64
    )
    head: int
    count: int
    seq_num: int

    @classmethod
    def from_decoded(cls, obj: Container) -> "EventQueueHeader":
        return cls(head=obj.head, count=obj.count, seq_num=obj.seq_num)

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"head": self.head, "count": self.count, "seq_num": self.seq_num}

    def to_json(self) -> EventQueueHeaderJSON:
        return {"head": self.head, "count": self.count, "seq_num": self.seq_num}

    @classmethod
    def from_json(cls, obj: EventQueueHeaderJSON) -> "EventQueueHeader":
        return cls(head=obj["head"], count=obj["count"], seq_num=obj["seq_num"])
