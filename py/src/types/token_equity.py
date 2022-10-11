from __future__ import annotations
from . import (
    i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class TokenEquityJSON(typing.TypedDict):
    token_index: int
    value: i80f48.I80F48JSON


@dataclass
class TokenEquity:
    layout: typing.ClassVar = borsh.CStruct(
        "token_index" / borsh.U16, "value" / i80f48.I80F48.layout
    )
    token_index: int
    value: i80f48.I80F48

    @classmethod
    def from_decoded(cls, obj: Container) -> "TokenEquity":
        return cls(
            token_index=obj.token_index, value=i80f48.I80F48.from_decoded(obj.value)
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"token_index": self.token_index, "value": self.value.to_encodable()}

    def to_json(self) -> TokenEquityJSON:
        return {"token_index": self.token_index, "value": self.value.to_json()}

    @classmethod
    def from_json(cls, obj: TokenEquityJSON) -> "TokenEquity":
        return cls(
            token_index=obj["token_index"], value=i80f48.I80F48.from_json(obj["value"])
        )
