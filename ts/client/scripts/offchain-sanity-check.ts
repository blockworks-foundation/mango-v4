import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fetch from 'node-fetch';
import { Group } from '../src/accounts/group';
import { MangoAccount } from '../src/accounts/mangoAccount';
import { PerpMarketIndex } from '../src/accounts/perp';
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
    .filter((a) => toUiDecimalsForQuote(a.getEquity(group)) > 1000);

  console.log(`Table 1: On-chain vs Offchain Pnl`);
  console.log(
    `${'Account'.padStart(48)} ${'Equity'.padStart(
      16,
    )} ${'Net deposits'.padStart(16)} ${'On-chain PnL'.padStart(
      16,
    )} ${'Off-chain PnL'.padStart(16)} ${'Off-chain PnL Date'.padStart(
      16,
    )} ${'Diff'.padStart(16)}`,
  );
  for (const a of largeMangoAccounts) {
    const url = `https://api.mngo.cloud/data/v4/stats/performance_account?mango-account=${a.publicKey}&start-date=2023-04-29`;
    const resp = await fetch(url);
    const data = await resp.json();
    const keys = Object.keys(data).sort(
      (a, b) => new Date(a).getTime() - new Date(b).getTime(),
    );
    const latestEntry = keys.length - 1;
    const offChainPnl = data[keys[latestEntry]].pnl;

    const onChain = new BN(a.getEquity(group).floor().toNumber()).sub(
      a.netDeposits,
    );
    const diff = toUiDecimalsForQuote(onChain) - offChainPnl;
    console.log(
      `${a.publicKey.toString().padStart(48)} ${toUiDecimalsForQuote(
        a.getEquity(group),
      )
        .toLocaleString()
        .padStart(15)}$ ${toUiDecimalsForQuote(a.netDeposits)
        .toLocaleString()
        .padStart(15)}$ ${toUiDecimalsForQuote(onChain)
        .toLocaleString()
        .padStart(15)}$ ${offChainPnl.toLocaleString().padStart(15)}$ (${
        keys[keys.length - 1]
      }) ${diff.toLocaleString().padStart(15)}$`,
    );
  }
  console.log();

  console.log(`Table 2: On-chain vs Offchain Cumulating funding`);
  console.log(
    `${'Account'.padStart(48)} ${'BTC On-chain'.padStart(
      11,
    )} ${'BTC Off-chain'.padStart(11)} ${'diff'.padStart(11)}`,
  );
  function funding(
    group: Group,
    marketIndex: PerpMarketIndex,
    mangoAccount: MangoAccount,
    data: any,
  ): number[] {
    const pps = mangoAccount
      .perpActive()
      .filter((a) => a.marketIndex === marketIndex);
    if (pps.length == 0) {
      return [-1, -1];
    }

    const pp = pps[0];
    const onChain = toUiDecimalsForQuote(
      pp.cumulativeLongFunding - pp.cumulativeShortFunding,
    );
    const offChainEntry =
      data[
        group.getPerpMarketByMarketIndex(marketIndex as PerpMarketIndex).name
      ];
    if (!offChainEntry) {
      return [onChain, -1];
    }

    const offChain =
      offChainEntry['long_funding'] + offChainEntry['short_funding'];
    return [onChain, offChain];
  }
  for (const a of largeMangoAccounts) {
    const resp = await fetch(
      `https://api.mngo.cloud/data/v4/stats/funding-account-total?mango-account=${a.publicKey}`,
    );
    const data = await resp.json();

    const btc = funding(group, 0 as PerpMarketIndex, a, data);
    // const sol = funding(group, 2 as PerpMarketIndex, a, data);
    // const eth = funding(group, 3 as PerpMarketIndex, a, data);
    if (btc[0] != -1 && btc[1] != -1)
      console.log(
        `${a.publicKey.toString().padStart(48)} ${btc[0]
          .toLocaleString()
          .padStart(10)}$ ${btc[1].toLocaleString().padStart(10)}$ ${(
          btc[0] - btc[1]
        )
          .toLocaleString()
          .padStart(10)}$`,
      );
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
