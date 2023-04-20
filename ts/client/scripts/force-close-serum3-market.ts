import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import range from 'lodash/range';
import { MarketIndex } from '../src/accounts/serum3';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_PK =
  process.env.GROUP_PK || '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';
const MARKET_INDEX = Number(process.env.MARKET_INDEX) as MarketIndex;

async function forceCloseSerum3Market(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const serum3Market = group.serum3MarketsMapByMarketIndex.get(MARKET_INDEX)!;
  if (!serum3Market.reduceOnly) {
    throw new Error(`Unexpected reduce only state ${serum3Market.reduceOnly}`);
  }
  if (!serum3Market.forceClose) {
    throw new Error(`Unexpected force close state ${serum3Market.forceClose}`);
  }

  // Get all mango accounts who have a serum oo account for the given market
  const mangoAccounts = (await client.getAllMangoAccounts(group, true)).filter(
    (a) => a.serum3OosMapByMarketIndex.get(MARKET_INDEX) !== undefined,
  );

  for (let a of mangoAccounts) {
    // Cancel all orders and confirm that all have been cancelled
    for (const _ of range(0, 10)) {
      console.log(a.getSerum3OoAccount(MARKET_INDEX).freeSlotBits.zeroBits());
      const sig = await client.serum3LiqForceCancelOrders(
        group,
        a,
        serum3Market.serumMarketExternal,
        10,
      );
      console.log(
        ` serum3LiqForceCancelOrders for ${
          a.publicKey
        }, sig https://explorer.solana.com/tx/${sig}?cluster=${
          CLUSTER == 'devnet' ? 'devnet' : ''
        }`,
      );
      a = await a.reload(client);
      if (a.getSerum3OoAccount(MARKET_INDEX).freeSlotBits.zeroBits() === 0) {
        break;
      }
    }
  }
}

forceCloseSerum3Market();
