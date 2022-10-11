from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class UninitializedJSON(typing.TypedDict):
    kind: typing.Literal["Uninitialized"]


class InnerNodeJSON(typing.TypedDict):
    kind: typing.Literal["InnerNode"]


class LeafNodeJSON(typing.TypedDict):
    kind: typing.Literal["LeafNode"]


class FreeNodeJSON(typing.TypedDict):
    kind: typing.Literal["FreeNode"]


class LastFreeNodeJSON(typing.TypedDict):
    kind: typing.Literal["LastFreeNode"]


@dataclass
class Uninitialized:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Uninitialized"

    @classmethod
    def to_json(cls) -> UninitializedJSON:
        return UninitializedJSON(
            kind="Uninitialized",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Uninitialized": {},
        }


@dataclass
class InnerNode:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "InnerNode"

    @classmethod
    def to_json(cls) -> InnerNodeJSON:
        return InnerNodeJSON(
            kind="InnerNode",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "InnerNode": {},
        }


@dataclass
class LeafNode:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "LeafNode"

    @classmethod
    def to_json(cls) -> LeafNodeJSON:
        return LeafNodeJSON(
            kind="LeafNode",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "LeafNode": {},
        }


@dataclass
class FreeNode:
    discriminator: typing.ClassVar = 3
    kind: typing.ClassVar = "FreeNode"

    @classmethod
    def to_json(cls) -> FreeNodeJSON:
        return FreeNodeJSON(
            kind="FreeNode",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "FreeNode": {},
        }


@dataclass
class LastFreeNode:
    discriminator: typing.ClassVar = 4
    kind: typing.ClassVar = "LastFreeNode"

    @classmethod
    def to_json(cls) -> LastFreeNodeJSON:
        return LastFreeNodeJSON(
            kind="LastFreeNode",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "LastFreeNode": {},
        }


NodeTagKind = typing.Union[Uninitialized, InnerNode, LeafNode, FreeNode, LastFreeNode]
NodeTagJSON = typing.Union[
    UninitializedJSON, InnerNodeJSON, LeafNodeJSON, FreeNodeJSON, LastFreeNodeJSON
]


def from_decoded(obj: dict) -> NodeTagKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Uninitialized" in obj:
        return Uninitialized()
    if "InnerNode" in obj:
        return InnerNode()
    if "LeafNode" in obj:
        return LeafNode()
    if "FreeNode" in obj:
        return FreeNode()
    if "LastFreeNode" in obj:
        return LastFreeNode()
    raise ValueError("Invalid enum object")


def from_json(obj: NodeTagJSON) -> NodeTagKind:
    if obj["kind"] == "Uninitialized":
        return Uninitialized()
    if obj["kind"] == "InnerNode":
        return InnerNode()
    if obj["kind"] == "LeafNode":
        return LeafNode()
    if obj["kind"] == "FreeNode":
        return FreeNode()
    if obj["kind"] == "LastFreeNode":
        return LastFreeNode()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Uninitialized" / borsh.CStruct(),
    "InnerNode" / borsh.CStruct(),
    "LeafNode" / borsh.CStruct(),
    "FreeNode" / borsh.CStruct(),
    "LastFreeNode" / borsh.CStruct(),
)
