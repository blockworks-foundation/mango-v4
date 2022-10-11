from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class LimitJSON(typing.TypedDict):
    kind: typing.Literal["Limit"]


class ImmediateOrCancelJSON(typing.TypedDict):
    kind: typing.Literal["ImmediateOrCancel"]


class PostOnlyJSON(typing.TypedDict):
    kind: typing.Literal["PostOnly"]


@dataclass
class Limit:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Limit"

    @classmethod
    def to_json(cls) -> LimitJSON:
        return LimitJSON(
            kind="Limit",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Limit": {},
        }


@dataclass
class ImmediateOrCancel:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "ImmediateOrCancel"

    @classmethod
    def to_json(cls) -> ImmediateOrCancelJSON:
        return ImmediateOrCancelJSON(
            kind="ImmediateOrCancel",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "ImmediateOrCancel": {},
        }


@dataclass
class PostOnly:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "PostOnly"

    @classmethod
    def to_json(cls) -> PostOnlyJSON:
        return PostOnlyJSON(
            kind="PostOnly",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "PostOnly": {},
        }


Serum3OrderTypeKind = typing.Union[Limit, ImmediateOrCancel, PostOnly]
Serum3OrderTypeJSON = typing.Union[LimitJSON, ImmediateOrCancelJSON, PostOnlyJSON]


def from_decoded(obj: dict) -> Serum3OrderTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Limit" in obj:
        return Limit()
    if "ImmediateOrCancel" in obj:
        return ImmediateOrCancel()
    if "PostOnly" in obj:
        return PostOnly()
    raise ValueError("Invalid enum object")


def from_json(obj: Serum3OrderTypeJSON) -> Serum3OrderTypeKind:
    if obj["kind"] == "Limit":
        return Limit()
    if obj["kind"] == "ImmediateOrCancel":
        return ImmediateOrCancel()
    if obj["kind"] == "PostOnly":
        return PostOnly()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Limit" / borsh.CStruct(),
    "ImmediateOrCancel" / borsh.CStruct(),
    "PostOnly" / borsh.CStruct(),
)
