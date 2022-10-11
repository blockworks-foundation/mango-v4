from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class UnknownJSON(typing.TypedDict):
    kind: typing.Literal["Unknown"]


class LiqTokenBankruptcyJSON(typing.TypedDict):
    kind: typing.Literal["LiqTokenBankruptcy"]


class LiqTokenWithTokenJSON(typing.TypedDict):
    kind: typing.Literal["LiqTokenWithToken"]


class Serum3LiqForceCancelOrdersJSON(typing.TypedDict):
    kind: typing.Literal["Serum3LiqForceCancelOrders"]


class Serum3PlaceOrderJSON(typing.TypedDict):
    kind: typing.Literal["Serum3PlaceOrder"]


class Serum3SettleFundsJSON(typing.TypedDict):
    kind: typing.Literal["Serum3SettleFunds"]


class TokenWithdrawJSON(typing.TypedDict):
    kind: typing.Literal["TokenWithdraw"]


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
class LiqTokenBankruptcy:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "LiqTokenBankruptcy"

    @classmethod
    def to_json(cls) -> LiqTokenBankruptcyJSON:
        return LiqTokenBankruptcyJSON(
            kind="LiqTokenBankruptcy",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "LiqTokenBankruptcy": {},
        }


@dataclass
class LiqTokenWithToken:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "LiqTokenWithToken"

    @classmethod
    def to_json(cls) -> LiqTokenWithTokenJSON:
        return LiqTokenWithTokenJSON(
            kind="LiqTokenWithToken",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "LiqTokenWithToken": {},
        }


@dataclass
class Serum3LiqForceCancelOrders:
    discriminator: typing.ClassVar = 3
    kind: typing.ClassVar = "Serum3LiqForceCancelOrders"

    @classmethod
    def to_json(cls) -> Serum3LiqForceCancelOrdersJSON:
        return Serum3LiqForceCancelOrdersJSON(
            kind="Serum3LiqForceCancelOrders",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Serum3LiqForceCancelOrders": {},
        }


@dataclass
class Serum3PlaceOrder:
    discriminator: typing.ClassVar = 4
    kind: typing.ClassVar = "Serum3PlaceOrder"

    @classmethod
    def to_json(cls) -> Serum3PlaceOrderJSON:
        return Serum3PlaceOrderJSON(
            kind="Serum3PlaceOrder",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Serum3PlaceOrder": {},
        }


@dataclass
class Serum3SettleFunds:
    discriminator: typing.ClassVar = 5
    kind: typing.ClassVar = "Serum3SettleFunds"

    @classmethod
    def to_json(cls) -> Serum3SettleFundsJSON:
        return Serum3SettleFundsJSON(
            kind="Serum3SettleFunds",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Serum3SettleFunds": {},
        }


@dataclass
class TokenWithdraw:
    discriminator: typing.ClassVar = 6
    kind: typing.ClassVar = "TokenWithdraw"

    @classmethod
    def to_json(cls) -> TokenWithdrawJSON:
        return TokenWithdrawJSON(
            kind="TokenWithdraw",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "TokenWithdraw": {},
        }


LoanOriginationFeeInstructionKind = typing.Union[
    Unknown,
    LiqTokenBankruptcy,
    LiqTokenWithToken,
    Serum3LiqForceCancelOrders,
    Serum3PlaceOrder,
    Serum3SettleFunds,
    TokenWithdraw,
]
LoanOriginationFeeInstructionJSON = typing.Union[
    UnknownJSON,
    LiqTokenBankruptcyJSON,
    LiqTokenWithTokenJSON,
    Serum3LiqForceCancelOrdersJSON,
    Serum3PlaceOrderJSON,
    Serum3SettleFundsJSON,
    TokenWithdrawJSON,
]


def from_decoded(obj: dict) -> LoanOriginationFeeInstructionKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Unknown" in obj:
        return Unknown()
    if "LiqTokenBankruptcy" in obj:
        return LiqTokenBankruptcy()
    if "LiqTokenWithToken" in obj:
        return LiqTokenWithToken()
    if "Serum3LiqForceCancelOrders" in obj:
        return Serum3LiqForceCancelOrders()
    if "Serum3PlaceOrder" in obj:
        return Serum3PlaceOrder()
    if "Serum3SettleFunds" in obj:
        return Serum3SettleFunds()
    if "TokenWithdraw" in obj:
        return TokenWithdraw()
    raise ValueError("Invalid enum object")


def from_json(
    obj: LoanOriginationFeeInstructionJSON,
) -> LoanOriginationFeeInstructionKind:
    if obj["kind"] == "Unknown":
        return Unknown()
    if obj["kind"] == "LiqTokenBankruptcy":
        return LiqTokenBankruptcy()
    if obj["kind"] == "LiqTokenWithToken":
        return LiqTokenWithToken()
    if obj["kind"] == "Serum3LiqForceCancelOrders":
        return Serum3LiqForceCancelOrders()
    if obj["kind"] == "Serum3PlaceOrder":
        return Serum3PlaceOrder()
    if obj["kind"] == "Serum3SettleFunds":
        return Serum3SettleFunds()
    if obj["kind"] == "TokenWithdraw":
        return TokenWithdraw()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Unknown" / borsh.CStruct(),
    "LiqTokenBankruptcy" / borsh.CStruct(),
    "LiqTokenWithToken" / borsh.CStruct(),
    "Serum3LiqForceCancelOrders" / borsh.CStruct(),
    "Serum3PlaceOrder" / borsh.CStruct(),
    "Serum3SettleFunds" / borsh.CStruct(),
    "TokenWithdraw" / borsh.CStruct(),
)
