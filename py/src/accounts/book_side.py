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
from ..program_id import PROGRAM_ID
from .. import types


class BookSideJSON(typing.TypedDict):
    book_side_type: types.book_side_type.BookSideTypeJSON
    padding: list[int]
    bump_index: int
    free_list_len: int
    free_list_head: int
    root_node: int
    leaf_count: int
    nodes: list[types.any_node.AnyNodeJSON]
    reserved: list[int]


@dataclass
class BookSide:
    discriminator: typing.ClassVar = b"H,\xe1\x8d\xb2\x82a9"
    layout: typing.ClassVar = borsh.CStruct(
        "book_side_type" / types.book_side_type.layout,
        "padding" / borsh.U8[3],
        "bump_index" / borsh.U32,
        "free_list_len" / borsh.U32,
        "free_list_head" / borsh.U32,
        "root_node" / borsh.U32,
        "leaf_count" / borsh.U32,
        "nodes" / types.any_node.AnyNode.layout[1024],
        "reserved" / borsh.U8[256],
    )
    book_side_type: types.book_side_type.BookSideTypeKind
    padding: list[int]
    bump_index: int
    free_list_len: int
    free_list_head: int
    root_node: int
    leaf_count: int
    nodes: list[types.any_node.AnyNode]
    reserved: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: PublicKey,
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.Optional["BookSide"]:
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
    ) -> typing.List[typing.Optional["BookSide"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["BookSide"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "BookSide":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = BookSide.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            book_side_type=types.book_side_type.from_decoded(dec.book_side_type),
            padding=dec.padding,
            bump_index=dec.bump_index,
            free_list_len=dec.free_list_len,
            free_list_head=dec.free_list_head,
            root_node=dec.root_node,
            leaf_count=dec.leaf_count,
            nodes=list(
                map(lambda item: types.any_node.AnyNode.from_decoded(item), dec.nodes)
            ),
            reserved=dec.reserved,
        )

    def to_json(self) -> BookSideJSON:
        return {
            "book_side_type": self.book_side_type.to_json(),
            "padding": self.padding,
            "bump_index": self.bump_index,
            "free_list_len": self.free_list_len,
            "free_list_head": self.free_list_head,
            "root_node": self.root_node,
            "leaf_count": self.leaf_count,
            "nodes": list(map(lambda item: item.to_json(), self.nodes)),
            "reserved": self.reserved,
        }

    @classmethod
    def from_json(cls, obj: BookSideJSON) -> "BookSide":
        return cls(
            book_side_type=types.book_side_type.from_json(obj["book_side_type"]),
            padding=obj["padding"],
            bump_index=obj["bump_index"],
            free_list_len=obj["free_list_len"],
            free_list_head=obj["free_list_head"],
            root_node=obj["root_node"],
            leaf_count=obj["leaf_count"],
            nodes=list(
                map(lambda item: types.any_node.AnyNode.from_json(item), obj["nodes"])
            ),
            reserved=obj["reserved"],
        )
