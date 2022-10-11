import typing
from dataclasses import dataclass
from base64 import b64decode
from solana.publickey import PublicKey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Commitment
import borsh_construct as borsh
from anchorpy.coder.accounts import ACCOUNT_DISCRIMINATOR_SIZE
from anchorpy.error import AccountInvalidDiscriminator
from anchorpy.utils.rpc import get_multiple_accounts
from anchorpy.borsh_extension import BorshPubkey
from ..program_id import PROGRAM_ID


class Serum3MarketIndexReservationJSON(typing.TypedDict):
    group: str
    market_index: int
    reserved: list[int]


@dataclass
class Serum3MarketIndexReservation:
    discriminator: typing.ClassVar = b"\xf6\x10\xc6d\xefpx5"
    layout: typing.ClassVar = borsh.CStruct(
        "group" / BorshPubkey, "market_index" / borsh.U16, "reserved" / borsh.U8[38]
    )
    group: PublicKey
    market_index: int
    reserved: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: PublicKey,
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.Optional["Serum3MarketIndexReservation"]:
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
    ) -> typing.List[typing.Optional["Serum3MarketIndexReservation"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["Serum3MarketIndexReservation"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "Serum3MarketIndexReservation":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = Serum3MarketIndexReservation.layout.parse(
            data[ACCOUNT_DISCRIMINATOR_SIZE:]
        )
        return cls(
            group=dec.group,
            market_index=dec.market_index,
            reserved=dec.reserved,
        )

    def to_json(self) -> Serum3MarketIndexReservationJSON:
        return {
            "group": str(self.group),
            "market_index": self.market_index,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(
        cls, obj: Serum3MarketIndexReservationJSON
    ) -> "Serum3MarketIndexReservation":
        return cls(
            group=PublicKey(obj["group"]),
            market_index=obj["market_index"],
            reserved=obj["reserved"],
        )
