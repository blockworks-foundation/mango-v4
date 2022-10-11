from __future__ import annotations
from . import (
    token_info,
    serum3_info,
    perp_info,
)
import typing
from dataclasses import dataclass
from construct import Container, Construct
import borsh_construct as borsh


class HealthCacheJSON(typing.TypedDict):
    token_infos: list[token_info.TokenInfoJSON]
    serum3_infos: list[serum3_info.Serum3InfoJSON]
    perp_infos: list[perp_info.PerpInfoJSON]
    being_liquidated: bool


@dataclass
class HealthCache:
    layout: typing.ClassVar = borsh.CStruct(
        "token_infos" / borsh.Vec(typing.cast(Construct, token_info.TokenInfo.layout)),
        "serum3_infos"
        / borsh.Vec(typing.cast(Construct, serum3_info.Serum3Info.layout)),
        "perp_infos" / borsh.Vec(typing.cast(Construct, perp_info.PerpInfo.layout)),
        "being_liquidated" / borsh.Bool,
    )
    token_infos: list[token_info.TokenInfo]
    serum3_infos: list[serum3_info.Serum3Info]
    perp_infos: list[perp_info.PerpInfo]
    being_liquidated: bool

    @classmethod
    def from_decoded(cls, obj: Container) -> "HealthCache":
        return cls(
            token_infos=list(
                map(
                    lambda item: token_info.TokenInfo.from_decoded(item),
                    obj.token_infos,
                )
            ),
            serum3_infos=list(
                map(
                    lambda item: serum3_info.Serum3Info.from_decoded(item),
                    obj.serum3_infos,
                )
            ),
            perp_infos=list(
                map(lambda item: perp_info.PerpInfo.from_decoded(item), obj.perp_infos)
            ),
            being_liquidated=obj.being_liquidated,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "token_infos": list(
                map(lambda item: item.to_encodable(), self.token_infos)
            ),
            "serum3_infos": list(
                map(lambda item: item.to_encodable(), self.serum3_infos)
            ),
            "perp_infos": list(map(lambda item: item.to_encodable(), self.perp_infos)),
            "being_liquidated": self.being_liquidated,
        }

    def to_json(self) -> HealthCacheJSON:
        return {
            "token_infos": list(map(lambda item: item.to_json(), self.token_infos)),
            "serum3_infos": list(map(lambda item: item.to_json(), self.serum3_infos)),
            "perp_infos": list(map(lambda item: item.to_json(), self.perp_infos)),
            "being_liquidated": self.being_liquidated,
        }

    @classmethod
    def from_json(cls, obj: HealthCacheJSON) -> "HealthCache":
        return cls(
            token_infos=list(
                map(
                    lambda item: token_info.TokenInfo.from_json(item),
                    obj["token_infos"],
                )
            ),
            serum3_infos=list(
                map(
                    lambda item: serum3_info.Serum3Info.from_json(item),
                    obj["serum3_infos"],
                )
            ),
            perp_infos=list(
                map(lambda item: perp_info.PerpInfo.from_json(item), obj["perp_infos"])
            ),
            being_liquidated=obj["being_liquidated"],
        )
