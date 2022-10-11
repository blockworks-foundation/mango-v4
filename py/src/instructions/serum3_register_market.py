from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class Serum3RegisterMarketArgs(typing.TypedDict):
    market_index: int
    name: str


layout = borsh.CStruct("market_index" / borsh.U16, "name" / borsh.String)


class Serum3RegisterMarketAccounts(typing.TypedDict):
    group: PublicKey
    admin: PublicKey
    serum_program: PublicKey
    serum_market_external: PublicKey
    serum_market: PublicKey
    index_reservation: PublicKey
    quote_bank: PublicKey
    base_bank: PublicKey
    payer: PublicKey
    system_program: PublicKey


def serum3_register_market(
    args: Serum3RegisterMarketArgs,
    accounts: Serum3RegisterMarketAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["serum_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_market_external"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["serum_market"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["index_reservation"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=accounts["quote_bank"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["base_bank"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["payer"], is_signer=True, is_writable=True),
        AccountMeta(
            pubkey=accounts["system_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"(\x0emx\xde\x9c\xd1\x00"
    encoded_args = layout.build(
        {
            "market_index": args["market_index"],
            "name": args["name"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
