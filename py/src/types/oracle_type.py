from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class PythJSON(typing.TypedDict):
    kind: typing.Literal["Pyth"]


class StubJSON(typing.TypedDict):
    kind: typing.Literal["Stub"]


class SwitchboardV1JSON(typing.TypedDict):
    kind: typing.Literal["SwitchboardV1"]


class SwitchboardV2JSON(typing.TypedDict):
    kind: typing.Literal["SwitchboardV2"]


@dataclass
class Pyth:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Pyth"

    @classmethod
    def to_json(cls) -> PythJSON:
        return PythJSON(
            kind="Pyth",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Pyth": {},
        }


@dataclass
class Stub:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Stub"

    @classmethod
    def to_json(cls) -> StubJSON:
        return StubJSON(
            kind="Stub",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Stub": {},
        }


@dataclass
class SwitchboardV1:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "SwitchboardV1"

    @classmethod
    def to_json(cls) -> SwitchboardV1JSON:
        return SwitchboardV1JSON(
            kind="SwitchboardV1",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "SwitchboardV1": {},
        }


@dataclass
class SwitchboardV2:
    discriminator: typing.ClassVar = 3
    kind: typing.ClassVar = "SwitchboardV2"

    @classmethod
    def to_json(cls) -> SwitchboardV2JSON:
        return SwitchboardV2JSON(
            kind="SwitchboardV2",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "SwitchboardV2": {},
        }


OracleTypeKind = typing.Union[Pyth, Stub, SwitchboardV1, SwitchboardV2]
OracleTypeJSON = typing.Union[PythJSON, StubJSON, SwitchboardV1JSON, SwitchboardV2JSON]


def from_decoded(obj: dict) -> OracleTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Pyth" in obj:
        return Pyth()
    if "Stub" in obj:
        return Stub()
    if "SwitchboardV1" in obj:
        return SwitchboardV1()
    if "SwitchboardV2" in obj:
        return SwitchboardV2()
    raise ValueError("Invalid enum object")


def from_json(obj: OracleTypeJSON) -> OracleTypeKind:
    if obj["kind"] == "Pyth":
        return Pyth()
    if obj["kind"] == "Stub":
        return Stub()
    if obj["kind"] == "SwitchboardV1":
        return SwitchboardV1()
    if obj["kind"] == "SwitchboardV2":
        return SwitchboardV2()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Pyth" / borsh.CStruct(),
    "Stub" / borsh.CStruct(),
    "SwitchboardV1" / borsh.CStruct(),
    "SwitchboardV2" / borsh.CStruct(),
)
