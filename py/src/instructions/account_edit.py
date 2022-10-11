from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class AccountEditArgs(typing.TypedDict):
    name_opt: typing.Optional[str]
    delegate_opt: typing.Optional[PublicKey]


layout = borsh.CStruct(
    "name_opt" / borsh.Option(borsh.String), "delegate_opt" / borsh.Option(BorshPubkey)
)


class AccountEditAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    owner: PublicKey


def account_edit(
    args: AccountEditArgs,
    accounts: AccountEditAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["owner"], is_signer=True, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xba\xd3\xcd\xb7s]\x18\xa1"
    encoded_args = layout.build(
        {
            "name_opt": args["name_opt"],
            "delegate_opt": args["delegate_opt"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
