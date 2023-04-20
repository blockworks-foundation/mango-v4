import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoAccount } from '../src/accounts/mangoAccount';
import { PerpMarketIndex } from '../src/accounts/perp';
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
const PERP_MARKET_INDEX = Number(
  process.env.PERP_MARKET_INDEX,
) as PerpMarketIndex;

async function forceClosePerpPositions(): Promise<void> {
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
  const mangoAccounts = (await client.getAllMangoAccounts(group)).filter(
    (a) => a.getPerpPosition(PERP_MARKET_INDEX) !== undefined,
  );
  mangoAccounts.sort(
    (a, b) =>
      b.getPerpPositionUi(group, PERP_MARKET_INDEX) -
      a.getPerpPositionUi(group, PERP_MARKET_INDEX),
  );

  let a: MangoAccount;
  let b: MangoAccount;
  let i = 0,
    j = mangoAccounts.length - 1;

  while (i < mangoAccounts.length - 1 && j > 0) {
    a = mangoAccounts[i];
    b = mangoAccounts[j];
    await client.perpForceClosePosition(group, PERP_MARKET_INDEX, a, b);
    a = await a.reload(client);
    b = await b.reload(client);
    if (b.getPerpPositionUi(group, PERP_MARKET_INDEX) === 0) {
      j--;
    }
    if (a.getPerpPositionUi(group, PERP_MARKET_INDEX) === 0) {
      i++;
    }
  }
}

forceClosePerpPositions();
