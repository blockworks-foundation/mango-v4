import { MANGO_V4_ID, MangoClient, sleep } from '../src';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_PK =
  process.env.GROUP_PK || '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';
const PERP_MARKET_NAME = process.env.PERP_MARKET_NAME || '';

const ERROR_ACCOUNTS = {
  'RENDER-PERP': [
    'Ganz6P5BdESMGpyFxe5Fupf6pED6vVGTJ6cAUheY8yE2',
    'G4Vjwbbrf61YswkDNjA1RUYDtWDLUSyS1tfHdQj7Gg2v',
  ],
  'SOL-PERP': [
    'Boq7FhAu9YLB3VswYtsyYDWDjJbMex4Vk2cHcY7czaQG',
    '3zpwwXsLSc6ZViYncfdf4iDU6g7hgcEcGLHMZXGtCTDJ',
    'DmorHvWBFq56RnhBEF883BPLEQzgiyJm3LvW6i6phEww',
    '2xfnDzaxxVxr2KyWTepG1ziBB1e3o68T5ogctwV94uNU',
    '5eRTPihWUp1Xr4mmgQSxHkjnKjsP7nkh3uWsXucZE8Ew',
    'D5JJ4NmxqA24bmbUaqX76GmDDuHJa9sxRfeEvyQDAnDg',
    '8KDWv3SpSEdn9W6w5yuzn1DjG2MWGYkCDCB2KtsXbA6p',
    '2jNT3yMj4atk5CFQ9Ggdso4xtKc9B7ddBBiAmmuFuf2a',
    'DGCoautww7Rwg2KmBtwxywGZ4hkRwZ6rrxkcT3jLyFWn',
    '3yaJPSbpo9Y3v5A1f1Ljpu1fQHtDTvw5kzekQkAPouvk',
    '35g7saydHzgPvUFDvnzBFNE96D5T4ntvD1cEQTNi9mQz',
  ],
  'ETH-PERP': [],
  'BTC-PERP': ['EeGQcSUW5NBJCCPRBmKoGocLk4MEJvDzX5X8DKDP8WuC'],
};

/**
 * This code is intended to be used one perp market at a time. After running for one perp market,
 * Run perp-sanity-check afterwards to see if the perp market has been zeroed out
 */
async function perpSettleUnmatched(): Promise<void> {
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
  const pm = group.getPerpMarketByName(PERP_MARKET_NAME);
  const PERP_MARKET_INDEX = pm.perpMarketIndex;
  // const pm = group.getPerpMarketByMarketIndex(PERP_MARKET_INDEX);
  console.log(pm.name);
  if (!pm.reduceOnly) {
    throw new Error(`Unexpected reduce only state ${pm.reduceOnly}`);
  }
  if (!pm.forceClose) {
    throw new Error(`Unexpected force close state ${pm.forceClose}`);
  }

  const mangoAccounts = await client.getAllMangoAccounts(group);
  const posPnlAccounts = mangoAccounts
    .filter((a) => {
      const pp = a.getPerpPosition(PERP_MARKET_INDEX);
      return pp && pp.getUnsettledPnlUi(pm) > 0;
    })
    .sort(
      (a, b) =>
        b.getPerpPosition(PERP_MARKET_INDEX)!.getUnsettledPnlUi(pm)! -
        a.getPerpPosition(PERP_MARKET_INDEX)!.getUnsettledPnlUi(pm)!,
    );
  if (posPnlAccounts.length) {
    throw new Error(
      `Found positive pnl accounts for ${PERP_MARKET_NAME}. Settle those first!`,
    );
  }

  const negPnlAccounts = mangoAccounts
    .filter((a) => {
      const pp = a.getPerpPosition(PERP_MARKET_INDEX);
      return pp && pp.getUnsettledPnlUi(pm) < 0;
    })
    .sort(
      (a, b) =>
        a.getPerpPosition(PERP_MARKET_INDEX)!.getUnsettledPnlUi(pm)! -
        b.getPerpPosition(PERP_MARKET_INDEX)!.getUnsettledPnlUi(pm)!,
    );
  for (const a of negPnlAccounts) {
    console.log(
      a.publicKey,
      a.getPerpPosition(PERP_MARKET_INDEX)?.getUnsettledPnlUi(pm),
    );
  }

  for (const account of negPnlAccounts) {
    let negPnl = account
      .getPerpPosition(PERP_MARKET_INDEX)!
      .getUnsettledPnlUi(pm);
    console.log(`Settling: ${account.publicKey} - ${negPnl}`);

    try {
      const sig = await client.perpSettleUnmatched(
        group,
        account,
        PERP_MARKET_INDEX,
        10000,
      );
      console.log(`sig - ${sig.signature}`);
    } catch (e) {
      console.log(e);
      continue;
    }
    await sleep(1000);
    await account.reload(client);
    negPnl = account.getPerpPosition(PERP_MARKET_INDEX)!.getUnsettledPnlUi(pm);
    console.log(`Settled: ${account.publicKey} - ${negPnl}`);
  }
}

perpSettleUnmatched();
