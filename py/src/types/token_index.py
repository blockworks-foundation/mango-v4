from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class TokenIndexJSON(typing.TypedDict):
    val: int


@dataclass
class TokenIndex:
    layout: typing.ClassVar = borsh.CStruct("val" / borsh.U16)
    val: int

    @classmethod
    def from_decoded(cls, obj: Container) -> "TokenIndex":
        return cls(val=obj.val)

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"val": self.val}

    def to_json(self) -> TokenIndexJSON:
        return {"val": self.val}

    @classmethod
    def from_json(cls, obj: TokenIndexJSON) -> "TokenIndex":
        return cls(val=obj["val"])
