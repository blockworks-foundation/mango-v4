from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class InitJSON(typing.TypedDict):
    kind: typing.Literal["Init"]


class MaintJSON(typing.TypedDict):
    kind: typing.Literal["Maint"]


@dataclass
class Init:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Init"

    @classmethod
    def to_json(cls) -> InitJSON:
        return InitJSON(
            kind="Init",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Init": {},
        }


@dataclass
class Maint:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Maint"

    @classmethod
    def to_json(cls) -> MaintJSON:
        return MaintJSON(
            kind="Maint",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Maint": {},
        }


HealthTypeKind = typing.Union[Init, Maint]
HealthTypeJSON = typing.Union[InitJSON, MaintJSON]


def from_decoded(obj: dict) -> HealthTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Init" in obj:
        return Init()
    if "Maint" in obj:
        return Maint()
    raise ValueError("Invalid enum object")


def from_json(obj: HealthTypeJSON) -> HealthTypeKind:
    if obj["kind"] == "Init":
        return Init()
    if obj["kind"] == "Maint":
        return Maint()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen("Init" / borsh.CStruct(), "Maint" / borsh.CStruct())
