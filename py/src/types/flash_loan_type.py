from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class UnknownJSON(typing.TypedDict):
    kind: typing.Literal["Unknown"]


class SwapJSON(typing.TypedDict):
    kind: typing.Literal["Swap"]


@dataclass
class Unknown:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Unknown"

    @classmethod
    def to_json(cls) -> UnknownJSON:
        return UnknownJSON(
            kind="Unknown",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Unknown": {},
        }


@dataclass
class Swap:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Swap"

    @classmethod
    def to_json(cls) -> SwapJSON:
        return SwapJSON(
            kind="Swap",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Swap": {},
        }


FlashLoanTypeKind = typing.Union[Unknown, Swap]
FlashLoanTypeJSON = typing.Union[UnknownJSON, SwapJSON]


def from_decoded(obj: dict) -> FlashLoanTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Unknown" in obj:
        return Unknown()
    if "Swap" in obj:
        return Swap()
    raise ValueError("Invalid enum object")


def from_json(obj: FlashLoanTypeJSON) -> FlashLoanTypeKind:
    if obj["kind"] == "Unknown":
        return Unknown()
    if obj["kind"] == "Swap":
        return Swap()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen("Unknown" / borsh.CStruct(), "Swap" / borsh.CStruct())
