from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class FillJSON(typing.TypedDict):
    kind: typing.Literal["Fill"]


class OutJSON(typing.TypedDict):
    kind: typing.Literal["Out"]


class LiquidateJSON(typing.TypedDict):
    kind: typing.Literal["Liquidate"]


@dataclass
class Fill:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Fill"

    @classmethod
    def to_json(cls) -> FillJSON:
        return FillJSON(
            kind="Fill",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Fill": {},
        }


@dataclass
class Out:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Out"

    @classmethod
    def to_json(cls) -> OutJSON:
        return OutJSON(
            kind="Out",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Out": {},
        }


@dataclass
class Liquidate:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "Liquidate"

    @classmethod
    def to_json(cls) -> LiquidateJSON:
        return LiquidateJSON(
            kind="Liquidate",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Liquidate": {},
        }


EventTypeKind = typing.Union[Fill, Out, Liquidate]
EventTypeJSON = typing.Union[FillJSON, OutJSON, LiquidateJSON]


def from_decoded(obj: dict) -> EventTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Fill" in obj:
        return Fill()
    if "Out" in obj:
        return Out()
    if "Liquidate" in obj:
        return Liquidate()
    raise ValueError("Invalid enum object")


def from_json(obj: EventTypeJSON) -> EventTypeKind:
    if obj["kind"] == "Fill":
        return Fill()
    if obj["kind"] == "Out":
        return Out()
    if obj["kind"] == "Liquidate":
        return Liquidate()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Fill" / borsh.CStruct(), "Out" / borsh.CStruct(), "Liquidate" / borsh.CStruct()
)
