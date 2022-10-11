from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class InterestRateParamsJSON(typing.TypedDict):
    util0: float
    rate0: float
    util1: float
    rate1: float
    max_rate: float
    adjustment_factor: float


@dataclass
class InterestRateParams:
    layout: typing.ClassVar = borsh.CStruct(
        "util0" / borsh.F32,
        "rate0" / borsh.F32,
        "util1" / borsh.F32,
        "rate1" / borsh.F32,
        "max_rate" / borsh.F32,
        "adjustment_factor" / borsh.F32,
    )
    util0: float
    rate0: float
    util1: float
    rate1: float
    max_rate: float
    adjustment_factor: float

    @classmethod
    def from_decoded(cls, obj: Container) -> "InterestRateParams":
        return cls(
            util0=obj.util0,
            rate0=obj.rate0,
            util1=obj.util1,
            rate1=obj.rate1,
            max_rate=obj.max_rate,
            adjustment_factor=obj.adjustment_factor,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "util0": self.util0,
            "rate0": self.rate0,
            "util1": self.util1,
            "rate1": self.rate1,
            "max_rate": self.max_rate,
            "adjustment_factor": self.adjustment_factor,
        }

    def to_json(self) -> InterestRateParamsJSON:
        return {
            "util0": self.util0,
            "rate0": self.rate0,
            "util1": self.util1,
            "rate1": self.rate1,
            "max_rate": self.max_rate,
            "adjustment_factor": self.adjustment_factor,
        }

    @classmethod
    def from_json(cls, obj: InterestRateParamsJSON) -> "InterestRateParams":
        return cls(
            util0=obj["util0"],
            rate0=obj["rate0"],
            util1=obj["util1"],
            rate1=obj["rate1"],
            max_rate=obj["max_rate"],
            adjustment_factor=obj["adjustment_factor"],
        )
