import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import { BN } from 'bn.js';
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
  const mangoAccounts = (await client.getAllMangoAccounts(group, true)).filter(
    (a) => a.serum3OosMapByMarketIndex.get(MARKET_INDEX) !== undefined,
  );

  for (let a of mangoAccounts) {
    for (const _ of range(0, 10)) {
      await client.serum3LiqForceCancelOrders(
        group,
        a,
        serum3Market.serumMarketExternal,
      );
      a = await a.reload(client);
      if (
        !a.getSerum3OoAccount(MARKET_INDEX).orders.some((o) => !o.eq(new BN(0)))
      ) {
        break;
      }
    }
  }
}

forceCloseSerum3Market();
