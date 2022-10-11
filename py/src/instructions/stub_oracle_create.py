from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class StubOracleCreateArgs(typing.TypedDict):
    price: types.i80f48.I80F48


layout = borsh.CStruct("price" / types.i80f48.I80F48.layout)


class StubOracleCreateAccounts(typing.TypedDict):
    group: PublicKey
    oracle: PublicKey
    admin: PublicKey
    mint: PublicKey
    payer: PublicKey
    system_program: PublicKey


def stub_oracle_create(
    args: StubOracleCreateArgs,
    accounts: StubOracleCreateAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["oracle"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["mint"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["payer"], is_signer=True, is_writable=True),
        AccountMeta(
            pubkey=accounts["system_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xac?eS\x8dL\xc7\xd8"
    encoded_args = layout.build(
        {
            "price": args["price"].to_encodable(),
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
