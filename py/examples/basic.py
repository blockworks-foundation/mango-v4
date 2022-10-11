import asyncio
import json
import os

from anchorpy import Provider, Wallet
from solana.keypair import Keypair
from solana.rpc.async_api import AsyncClient
from solana.transaction import Transaction, TransactionInstruction

from src.mango_client import configs, MangoClient


async def function() -> asyncio.coroutine:
    with open(os.environ["USER_KEYPAIR"], "r") as f:
        secret = json.load(f)
    kp = Keypair.from_secret_key(bytes(secret))

    url = os.environ["CLUSTER_URL"]
    wallet = Wallet(kp)
    connection = AsyncClient(url)
    config = configs["devnet"]
    provider = Provider(connection, wallet)
    client = MangoClient.from_config(config, provider)

    group = await client.get_group(config.group)
    print(group)
    mango_account = await client.get_mango_account_for_owner(
        config.group, kp.public_key, 0
    )
    print(mango_account)


asyncio.run(function())
