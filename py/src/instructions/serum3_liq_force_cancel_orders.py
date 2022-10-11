from __future__ import annotations
import typing
from solana.publickey import PublicKey
from solana.transaction import TransactionInstruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class Serum3LiqForceCancelOrdersArgs(typing.TypedDict):
    limit: int


layout = borsh.CStruct("limit" / borsh.U8)


class Serum3LiqForceCancelOrdersAccounts(typing.TypedDict):
    group: PublicKey
    account: PublicKey
    open_orders: PublicKey
    serum_market: PublicKey
    serum_program: PublicKey
    serum_market_external: PublicKey
    market_bids: PublicKey
    market_asks: PublicKey
    market_event_queue: PublicKey
    market_base_vault: PublicKey
    market_quote_vault: PublicKey
    market_vault_signer: PublicKey
    quote_bank: PublicKey
    quote_vault: PublicKey
    base_bank: PublicKey
    base_vault: PublicKey
    token_program: PublicKey


def serum3_liq_force_cancel_orders(
    args: Serum3LiqForceCancelOrdersArgs,
    accounts: Serum3LiqForceCancelOrdersAccounts,
    program_id: PublicKey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> TransactionInstruction:
    keys: list[AccountMeta] = [
        AccountMeta(pubkey=accounts["group"], is_signer=False, is_writable=False),
        AccountMeta(pubkey=accounts["account"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["open_orders"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["serum_market"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_program"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["serum_market_external"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=accounts["market_bids"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["market_asks"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["market_event_queue"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_base_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_quote_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["market_vault_signer"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["quote_bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["quote_vault"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["base_bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["base_vault"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["token_program"], is_signer=False, is_writable=False
        ),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\x1f\xaa_]X6\t\xe7"
    encoded_args = layout.build(
        {
            "limit": args["limit"],
        }
    )
    data = identifier + encoded_args
    return TransactionInstruction(keys, program_id, data)
