import typing
from dataclasses import dataclass
from base64 import b64decode
from construct import Construct
from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Commitment
import borsh_construct as borsh
from anchorpy.coder.accounts import ACCOUNT_DISCRIMINATOR_SIZE
from anchorpy.error import AccountInvalidDiscriminator
from anchorpy.utils.rpc import get_multiple_accounts
from anchorpy.borsh_extension import BorshPubkey
from ..program_id import PROGRAM_ID
from .. import types


class MangoAccountJSON(typing.TypedDict):
    group: str
    owner: str
    name: list[int]
    delegate: str
    account_num: int
    being_liquidated: int
    in_health_region: int
    bump: int
    padding: list[int]
    net_deposits: int
    net_settled: int
    health_region_begin_init_health: int
    reserved: list[int]
    header_version: int
    padding3: list[int]
    padding4: int
    tokens: list[types.token_position.TokenPositionJSON]
    padding5: int
    serum3: list[types.serum3_orders.Serum3OrdersJSON]
    padding6: int
    perps: list[types.perp_position.PerpPositionJSON]
    padding7: int
    perp_open_orders: list[types.perp_open_order.PerpOpenOrderJSON]


@dataclass
class MangoAccount:
    discriminator: typing.ClassVar = b"\xf3\xe4\xf7\x03\xa94\xaf\x1f"
    layout: typing.ClassVar = borsh.CStruct(
        "group" / BorshPubkey,
        "owner" / BorshPubkey,
        "name" / borsh.U8[32],
        "delegate" / BorshPubkey,
        "account_num" / borsh.U32,
        "being_liquidated" / borsh.U8,
        "in_health_region" / borsh.U8,
        "bump" / borsh.U8,
        "padding" / borsh.U8[1],
        "net_deposits" / borsh.I64,
        "net_settled" / borsh.I64,
        "health_region_begin_init_health" / borsh.I64,
        "reserved" / borsh.U8[240],
        "header_version" / borsh.U8,
        "padding3" / borsh.U8[7],
        "padding4" / borsh.U32,
        "tokens"
        / borsh.Vec(typing.cast(Construct, types.token_position.TokenPosition.layout)),
        "padding5" / borsh.U32,
        "serum3"
        / borsh.Vec(typing.cast(Construct, types.serum3_orders.Serum3Orders.layout)),
        "padding6" / borsh.U32,
        "perps"
        / borsh.Vec(typing.cast(Construct, types.perp_position.PerpPosition.layout)),
        "padding7" / borsh.U32,
        "perp_open_orders"
        / borsh.Vec(typing.cast(Construct, types.perp_open_order.PerpOpenOrder.layout)),
    )
    group: PublicKey
    owner: PublicKey
    name: list[int]
    delegate: PublicKey
    account_num: int
    being_liquidated: int
    in_health_region: int
    bump: int
    padding: list[int]
    net_deposits: int
    net_settled: int
    health_region_begin_init_health: int
    reserved: list[int]
    header_version: int
    padding3: list[int]
    padding4: int
    tokens: list[types.token_position.TokenPosition]
    padding5: int
    serum3: list[types.serum3_orders.Serum3Orders]
    padding6: int
    perps: list[types.perp_position.PerpPosition]
    padding7: int
    perp_open_orders: list[types.perp_open_order.PerpOpenOrder]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: PublicKey,
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.Optional["MangoAccount"]:
        resp = await conn.get_account_info(address, commitment=commitment)
        info = resp["result"]["value"]
        if info is None:
            return None
        if info["owner"] != str(program_id):
            raise ValueError("Account does not belong to this program")
        bytes_data = b64decode(info["data"][0])
        return cls.decode(bytes_data)

    @classmethod
    async def fetch_multiple(
        cls,
        conn: AsyncClient,
        addresses: list[PublicKey],
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.List[typing.Optional["MangoAccount"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["MangoAccount"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "MangoAccount":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = MangoAccount.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            group=dec.group,
            owner=dec.owner,
            name=dec.name,
            delegate=dec.delegate,
            account_num=dec.account_num,
            being_liquidated=dec.being_liquidated,
            in_health_region=dec.in_health_region,
            bump=dec.bump,
            padding=dec.padding,
            net_deposits=dec.net_deposits,
            net_settled=dec.net_settled,
            health_region_begin_init_health=dec.health_region_begin_init_health,
            reserved=dec.reserved,
            header_version=dec.header_version,
            padding3=dec.padding3,
            padding4=dec.padding4,
            tokens=list(
                map(
                    lambda item: types.token_position.TokenPosition.from_decoded(item),
                    dec.tokens,
                )
            ),
            padding5=dec.padding5,
            serum3=list(
                map(
                    lambda item: types.serum3_orders.Serum3Orders.from_decoded(item),
                    dec.serum3,
                )
            ),
            padding6=dec.padding6,
            perps=list(
                map(
                    lambda item: types.perp_position.PerpPosition.from_decoded(item),
                    dec.perps,
                )
            ),
            padding7=dec.padding7,
            perp_open_orders=list(
                map(
                    lambda item: types.perp_open_order.PerpOpenOrder.from_decoded(item),
                    dec.perp_open_orders,
                )
            ),
        )

    def to_json(self) -> MangoAccountJSON:
        return {
            "group": str(self.group),
            "owner": str(self.owner),
            "name": self.name,
            "delegate": str(self.delegate),
            "account_num": self.account_num,
            "being_liquidated": self.being_liquidated,
            "in_health_region": self.in_health_region,
            "bump": self.bump,
            "padding": self.padding,
            "net_deposits": self.net_deposits,
            "net_settled": self.net_settled,
            "health_region_begin_init_health": self.health_region_begin_init_health,
            "reserved": self.reserved,
            "header_version": self.header_version,
            "padding3": self.padding3,
            "padding4": self.padding4,
            "tokens": list(map(lambda item: item.to_json(), self.tokens)),
            "padding5": self.padding5,
            "serum3": list(map(lambda item: item.to_json(), self.serum3)),
            "padding6": self.padding6,
            "perps": list(map(lambda item: item.to_json(), self.perps)),
            "padding7": self.padding7,
            "perp_open_orders": list(
                map(lambda item: item.to_json(), self.perp_open_orders)
            ),
        }

    @classmethod
    def from_json(cls, obj: MangoAccountJSON) -> "MangoAccount":
        return cls(
            group=PublicKey(obj["group"]),
            owner=PublicKey(obj["owner"]),
            name=obj["name"],
            delegate=PublicKey(obj["delegate"]),
            account_num=obj["account_num"],
            being_liquidated=obj["being_liquidated"],
            in_health_region=obj["in_health_region"],
            bump=obj["bump"],
            padding=obj["padding"],
            net_deposits=obj["net_deposits"],
            net_settled=obj["net_settled"],
            health_region_begin_init_health=obj["health_region_begin_init_health"],
            reserved=obj["reserved"],
            header_version=obj["header_version"],
            padding3=obj["padding3"],
            padding4=obj["padding4"],
            tokens=list(
                map(
                    lambda item: types.token_position.TokenPosition.from_json(item),
                    obj["tokens"],
                )
            ),
            padding5=obj["padding5"],
            serum3=list(
                map(
                    lambda item: types.serum3_orders.Serum3Orders.from_json(item),
                    obj["serum3"],
                )
            ),
            padding6=obj["padding6"],
            perps=list(
                map(
                    lambda item: types.perp_position.PerpPosition.from_json(item),
                    obj["perps"],
                )
            ),
            padding7=obj["padding7"],
            perp_open_orders=list(
                map(
                    lambda item: types.perp_open_order.PerpOpenOrder.from_json(item),
                    obj["perp_open_orders"],
                )
            ),
        )
