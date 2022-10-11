from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
from solana.publickey import PublicKey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class Serum3OrdersJSON(typing.TypedDict):
    open_orders: str
    base_borrows_without_fee: int
    quote_borrows_without_fee: int
    market_index: int
    base_token_index: int
    quote_token_index: int
    padding: list[int]
    reserved: list[int]


@dataclass
class Serum3Orders:
    layout: typing.ClassVar = borsh.CStruct(
        "open_orders" / BorshPubkey,
        "base_borrows_without_fee" / borsh.U64,
        "quote_borrows_without_fee" / borsh.U64,
        "market_index" / borsh.U16,
        "base_token_index" / borsh.U16,
        "quote_token_index" / borsh.U16,
        "padding" / borsh.U8[2],
        "reserved" / borsh.U8[64],
    )
    open_orders: PublicKey
    base_borrows_without_fee: int
    quote_borrows_without_fee: int
    market_index: int
    base_token_index: int
    quote_token_index: int
    padding: list[int]
    reserved: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "Serum3Orders":
        return cls(
            open_orders=obj.open_orders,
            base_borrows_without_fee=obj.base_borrows_without_fee,
            quote_borrows_without_fee=obj.quote_borrows_without_fee,
            market_index=obj.market_index,
            base_token_index=obj.base_token_index,
            quote_token_index=obj.quote_token_index,
            padding=obj.padding,
            reserved=obj.reserved,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "open_orders": self.open_orders,
            "base_borrows_without_fee": self.base_borrows_without_fee,
            "quote_borrows_without_fee": self.quote_borrows_without_fee,
            "market_index": self.market_index,
            "base_token_index": self.base_token_index,
            "quote_token_index": self.quote_token_index,
            "padding": self.padding,
            "reserved": self.reserved,
        }

    def to_json(self) -> Serum3OrdersJSON:
        return {
            "open_orders": str(self.open_orders),
            "base_borrows_without_fee": self.base_borrows_without_fee,
            "quote_borrows_without_fee": self.quote_borrows_without_fee,
            "market_index": self.market_index,
            "base_token_index": self.base_token_index,
            "quote_token_index": self.quote_token_index,
            "padding": self.padding,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: Serum3OrdersJSON) -> "Serum3Orders":
        return cls(
            open_orders=PublicKey(obj["open_orders"]),
            base_borrows_without_fee=obj["base_borrows_without_fee"],
            quote_borrows_without_fee=obj["quote_borrows_without_fee"],
            market_index=obj["market_index"],
            base_token_index=obj["base_token_index"],
            quote_token_index=obj["quote_token_index"],
            padding=obj["padding"],
            reserved=obj["reserved"],
        )
