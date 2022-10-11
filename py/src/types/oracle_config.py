from __future__ import annotations
from . import (
    i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class OracleConfigJSON(typing.TypedDict):
    conf_filter: i80f48.I80F48JSON


@dataclass
class OracleConfig:
    layout: typing.ClassVar = borsh.CStruct("conf_filter" / i80f48.I80F48.layout)
    conf_filter: i80f48.I80F48

    @classmethod
    def from_decoded(cls, obj: Container) -> "OracleConfig":
        return cls(conf_filter=i80f48.I80F48.from_decoded(obj.conf_filter))

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"conf_filter": self.conf_filter.to_encodable()}

    def to_json(self) -> OracleConfigJSON:
        return {"conf_filter": self.conf_filter.to_json()}

    @classmethod
    def from_json(cls, obj: OracleConfigJSON) -> "OracleConfig":
        return cls(conf_filter=i80f48.I80F48.from_json(obj["conf_filter"]))
