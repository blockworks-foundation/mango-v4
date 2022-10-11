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
from .. import types


class StubOracleJSON(typing.TypedDict):
    group: str
    mint: str
    price: types.i80f48.I80F48JSON
    last_updated: int
    reserved: list[int]


@dataclass
class StubOracle:
    discriminator: typing.ClassVar = b"\xe0\xfb\xfec\xb1\xae\x89\x04"
    layout: typing.ClassVar = borsh.CStruct(
        "group" / BorshPubkey,
        "mint" / BorshPubkey,
        "price" / types.i80f48.I80F48.layout,
        "last_updated" / borsh.I64,
        "reserved" / borsh.U8[128],
    )
    group: PublicKey
    mint: PublicKey
    price: types.i80f48.I80F48
    last_updated: int
    reserved: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: PublicKey,
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.Optional["StubOracle"]:
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
    ) -> typing.List[typing.Optional["StubOracle"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["StubOracle"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "StubOracle":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = StubOracle.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            group=dec.group,
            mint=dec.mint,
            price=types.i80f48.I80F48.from_decoded(dec.price),
            last_updated=dec.last_updated,
            reserved=dec.reserved,
        )

    def to_json(self) -> StubOracleJSON:
        return {
            "group": str(self.group),
            "mint": str(self.mint),
            "price": self.price.to_json(),
            "last_updated": self.last_updated,
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: StubOracleJSON) -> "StubOracle":
        return cls(
            group=PublicKey(obj["group"]),
            mint=PublicKey(obj["mint"]),
            price=types.i80f48.I80F48.from_json(obj["price"]),
            last_updated=obj["last_updated"],
            reserved=obj["reserved"],
        )
