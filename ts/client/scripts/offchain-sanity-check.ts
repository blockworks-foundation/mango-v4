import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fetch from 'node-fetch';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { toUiDecimalsForQuote } from '../src/utils';

dotenv.config();

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const GROUP_PK =
  process.env.GROUP_PK || '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  const wallet = new Wallet(new Keypair());
  const provider = new AnchorProvider(connection, wallet, options);
  const client = MangoClient.connect(provider, CLUSTER, MANGO_V4_ID[CLUSTER], {
    idsSource: 'get-program-accounts',
  });

  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const mangoAccounts = await client.getAllMangoAccounts(group, true);

  const largeMangoAccounts = mangoAccounts
    .sort((a, b) => b.getEquity(group).cmp(a.getEquity(group)))
    .slice(0, 20);

  console.log(
    `${'Account'.padStart(48)} ${'Equity'.padStart(
      15,
    )} ${'Net deposits'.padStart(15)} ${'On-chain PnL'.padStart(
      15,
    )} ${'Off-chain PnL'.padStart(15)} ${'Off-chain PnL Date'.padStart(15)}`,
  );
  largeMangoAccounts.forEach(async (a) => {
    const url = `https://api.mngo.cloud/data/v4/stats/performance_account?mango-account=${a.publicKey}&start-date=2023-04-29`;
    const resp = await fetch(url);
    const data = await resp.json();
    const keys = Object.keys(data).sort(
      (a, b) => new Date(a).getTime() - new Date(b).getTime(),
    );
    const offChainPnl = data[keys[keys.length - 1]].pnl;

    console.log(
      `${a.publicKey} ${toUiDecimalsForQuote(a.getEquity(group))
        .toLocaleString()
        .padStart(15)}$ ${toUiDecimalsForQuote(a.netDeposits)
        .toLocaleString()
        .padStart(15)}$ ${toUiDecimalsForQuote(
        new BN(a.getEquity(group).floor().toNumber()).sub(a.netDeposits),
      )
        .toLocaleString()
        .padStart(15)}$ ${offChainPnl.toLocaleString().padStart(15)}$ (${
        keys[keys.length - 1]
      })`,
    );
  });
}

try {
  main();
} catch (error) {
  console.log(error);
}
