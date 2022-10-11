from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class BidsJSON(typing.TypedDict):
    kind: typing.Literal["Bids"]


class AsksJSON(typing.TypedDict):
    kind: typing.Literal["Asks"]


@dataclass
class Bids:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Bids"

    @classmethod
    def to_json(cls) -> BidsJSON:
        return BidsJSON(
            kind="Bids",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Bids": {},
        }


@dataclass
class Asks:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Asks"

    @classmethod
    def to_json(cls) -> AsksJSON:
        return AsksJSON(
            kind="Asks",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Asks": {},
        }


BookSideTypeKind = typing.Union[Bids, Asks]
BookSideTypeJSON = typing.Union[BidsJSON, AsksJSON]


def from_decoded(obj: dict) -> BookSideTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Bids" in obj:
        return Bids()
    if "Asks" in obj:
        return Asks()
    raise ValueError("Invalid enum object")


def from_json(obj: BookSideTypeJSON) -> BookSideTypeKind:
    if obj["kind"] == "Bids":
        return Bids()
    if obj["kind"] == "Asks":
        return Asks()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen("Bids" / borsh.CStruct(), "Asks" / borsh.CStruct())
