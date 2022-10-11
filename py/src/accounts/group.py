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


class GroupJSON(typing.TypedDict):
    creator: str
    group_num: int
    admin: str
    fast_listing_admin: str
    padding: list[int]
    insurance_vault: str
    insurance_mint: str
    bump: int
    testing: int
    version: int
    padding2: list[int]
    address_lookup_tables: list[str]
    reserved: list[int]


@dataclass
class Group:
    discriminator: typing.ClassVar = b"\xd1\xf9\xd0?\xb6Y\xba\xfe"
    layout: typing.ClassVar = borsh.CStruct(
        "creator" / BorshPubkey,
        "group_num" / borsh.U32,
        "admin" / BorshPubkey,
        "fast_listing_admin" / BorshPubkey,
        "padding" / borsh.U8[4],
        "insurance_vault" / BorshPubkey,
        "insurance_mint" / BorshPubkey,
        "bump" / borsh.U8,
        "testing" / borsh.U8,
        "version" / borsh.U8,
        "padding2" / borsh.U8[5],
        "address_lookup_tables" / BorshPubkey[20],
        "reserved" / borsh.U8[1920],
    )
    creator: PublicKey
    group_num: int
    admin: PublicKey
    fast_listing_admin: PublicKey
    padding: list[int]
    insurance_vault: PublicKey
    insurance_mint: PublicKey
    bump: int
    testing: int
    version: int
    padding2: list[int]
    address_lookup_tables: list[PublicKey]
    reserved: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: PublicKey,
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.Optional["Group"]:
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
    ) -> typing.List[typing.Optional["Group"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["Group"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "Group":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = Group.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            creator=dec.creator,
            group_num=dec.group_num,
            admin=dec.admin,
            fast_listing_admin=dec.fast_listing_admin,
            padding=dec.padding,
            insurance_vault=dec.insurance_vault,
            insurance_mint=dec.insurance_mint,
            bump=dec.bump,
            testing=dec.testing,
            version=dec.version,
            padding2=dec.padding2,
            address_lookup_tables=dec.address_lookup_tables,
            reserved=dec.reserved,
        )

    def to_json(self) -> GroupJSON:
        return {
            "creator": str(self.creator),
            "group_num": self.group_num,
            "admin": str(self.admin),
            "fast_listing_admin": str(self.fast_listing_admin),
            "padding": self.padding,
            "insurance_vault": str(self.insurance_vault),
            "insurance_mint": str(self.insurance_mint),
            "bump": self.bump,
            "testing": self.testing,
            "version": self.version,
            "padding2": self.padding2,
            "address_lookup_tables": list(
                map(lambda item: str(item), self.address_lookup_tables)
            ),
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: GroupJSON) -> "Group":
        return cls(
            creator=PublicKey(obj["creator"]),
            group_num=obj["group_num"],
            admin=PublicKey(obj["admin"]),
            fast_listing_admin=PublicKey(obj["fast_listing_admin"]),
            padding=obj["padding"],
            insurance_vault=PublicKey(obj["insurance_vault"]),
            insurance_mint=PublicKey(obj["insurance_mint"]),
            bump=obj["bump"],
            testing=obj["testing"],
            version=obj["version"],
            padding2=obj["padding2"],
            address_lookup_tables=list(
                map(lambda item: PublicKey(item), obj["address_lookup_tables"])
            ),
            reserved=obj["reserved"],
        )
