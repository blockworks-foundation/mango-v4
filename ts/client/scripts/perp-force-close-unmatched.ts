import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MANGO_V4_ID, MangoClient, PerpMarketIndex, sleep } from '../src';

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

async function perpForceCloseUnmatched(): Promise<void> {
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
  const client = MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const pm = group.getPerpMarketByMarketIndex(PERP_MARKET_INDEX);
  console.log(pm.name);
  if (!pm.reduceOnly) {
    throw new Error(`Unexpected reduce only state ${pm.reduceOnly}`);
  }
  if (!pm.forceClose) {
    throw new Error(`Unexpected force close state ${pm.forceClose}`);
  }

  // Get all mango accounts who have a position in the given market
  const mangoAccounts = (await client.getAllMangoAccounts(group)).filter(
    (a) =>
      a.getPerpPosition(PERP_MARKET_INDEX) !== undefined &&
      a.getPerpPositionUi(group, PERP_MARKET_INDEX) !== 0,
  );

  // There are only two accounts, both with positive 1 base pos. Hit them both
  for (const mangoAccount of mangoAccounts) {
    console.log(
      `mangoAccount: ${mangoAccount.publicKey} pos: ${mangoAccount.getPerpPositionUi(group, PERP_MARKET_INDEX)}`,
    );
    const sig = await client.perpForceCloseUnmatched(
      group,
      PERP_MARKET_INDEX,
      mangoAccount,
    );
    console.log(`sig - ${sig.signature}`);
    await sleep(1000);
    await mangoAccount.reload(client);
    console.log(
      `mangoAccount: ${mangoAccount.publicKey} position: ${mangoAccount.getPerpPositionUi(group, PERP_MARKET_INDEX)}`,
    );
  }
}

perpForceCloseUnmatched();
