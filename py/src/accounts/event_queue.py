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


class EventQueueJSON(typing.TypedDict):
    header: types.event_queue_header.EventQueueHeaderJSON
    buf: list[types.any_event.AnyEventJSON]


@dataclass
class EventQueue:
    discriminator: typing.ClassVar = b")\xd0t\xd1\xadt\x8dD"
    layout: typing.ClassVar = borsh.CStruct(
        "header" / types.event_queue_header.EventQueueHeader.layout,
        "buf" / types.any_event.AnyEvent.layout[488],
    )
    header: types.event_queue_header.EventQueueHeader
    buf: list[types.any_event.AnyEvent]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: PublicKey,
        commitment: typing.Optional[Commitment] = None,
        program_id: PublicKey = PROGRAM_ID,
    ) -> typing.Optional["EventQueue"]:
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
    ) -> typing.List[typing.Optional["EventQueue"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["EventQueue"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "EventQueue":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = EventQueue.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            header=types.event_queue_header.EventQueueHeader.from_decoded(dec.header),
            buf=list(
                map(lambda item: types.any_event.AnyEvent.from_decoded(item), dec.buf)
            ),
        )

    def to_json(self) -> EventQueueJSON:
        return {
            "header": self.header.to_json(),
            "buf": list(map(lambda item: item.to_json(), self.buf)),
        }

    @classmethod
    def from_json(cls, obj: EventQueueJSON) -> "EventQueue":
        return cls(
            header=types.event_queue_header.EventQueueHeader.from_json(obj["header"]),
            buf=list(
                map(lambda item: types.any_event.AnyEvent.from_json(item), obj["buf"])
            ),
        )
