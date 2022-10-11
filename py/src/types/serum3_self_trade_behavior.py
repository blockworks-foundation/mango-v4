from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class DecrementTakeJSON(typing.TypedDict):
    kind: typing.Literal["DecrementTake"]


class CancelProvideJSON(typing.TypedDict):
    kind: typing.Literal["CancelProvide"]


class AbortTransactionJSON(typing.TypedDict):
    kind: typing.Literal["AbortTransaction"]


@dataclass
class DecrementTake:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "DecrementTake"

    @classmethod
    def to_json(cls) -> DecrementTakeJSON:
        return DecrementTakeJSON(
            kind="DecrementTake",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "DecrementTake": {},
        }


@dataclass
class CancelProvide:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "CancelProvide"

    @classmethod
    def to_json(cls) -> CancelProvideJSON:
        return CancelProvideJSON(
            kind="CancelProvide",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "CancelProvide": {},
        }


@dataclass
class AbortTransaction:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "AbortTransaction"

    @classmethod
    def to_json(cls) -> AbortTransactionJSON:
        return AbortTransactionJSON(
            kind="AbortTransaction",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "AbortTransaction": {},
        }


Serum3SelfTradeBehaviorKind = typing.Union[
    DecrementTake, CancelProvide, AbortTransaction
]
Serum3SelfTradeBehaviorJSON = typing.Union[
    DecrementTakeJSON, CancelProvideJSON, AbortTransactionJSON
]


def from_decoded(obj: dict) -> Serum3SelfTradeBehaviorKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "DecrementTake" in obj:
        return DecrementTake()
    if "CancelProvide" in obj:
        return CancelProvide()
    if "AbortTransaction" in obj:
        return AbortTransaction()
    raise ValueError("Invalid enum object")


def from_json(obj: Serum3SelfTradeBehaviorJSON) -> Serum3SelfTradeBehaviorKind:
    if obj["kind"] == "DecrementTake":
        return DecrementTake()
    if obj["kind"] == "CancelProvide":
        return CancelProvide()
    if obj["kind"] == "AbortTransaction":
        return AbortTransaction()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "DecrementTake" / borsh.CStruct(),
    "CancelProvide" / borsh.CStruct(),
    "AbortTransaction" / borsh.CStruct(),
)
