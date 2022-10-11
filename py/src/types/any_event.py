from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class AnyEventJSON(typing.TypedDict):
    event_type: int
    padding: list[int]


@dataclass
class AnyEvent:
    layout: typing.ClassVar = borsh.CStruct(
        "event_type" / borsh.U8, "padding" / borsh.U8[207]
    )
    event_type: int
    padding: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "AnyEvent":
        return cls(event_type=obj.event_type, padding=obj.padding)

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"event_type": self.event_type, "padding": self.padding}

    def to_json(self) -> AnyEventJSON:
        return {"event_type": self.event_type, "padding": self.padding}

    @classmethod
    def from_json(cls, obj: AnyEventJSON) -> "AnyEvent":
        return cls(event_type=obj["event_type"], padding=obj["padding"])
