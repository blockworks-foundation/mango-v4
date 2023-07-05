import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { BookSide, PerpMarket } from '../src/accounts/perp';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';

const { MB_CLUSTER_URL } = process.env;

const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

async function buildClient(): Promise<MangoClient> {
  const clientKeypair = new Keypair();

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const clientWallet = new Wallet(clientKeypair);
  const clientProvider = new AnchorProvider(connection, clientWallet, options);

  return await MangoClient.connect(
    clientProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );
}

function doBin(
  bs: BookSide,
  direction: 'bids' | 'asks',
  range: {
    start: number;
    end: number;
    scoreMultiplier: number;
    queueMultiplier: number;
  },
): Map<string, { size: number; score: number }> {
  const bin = new Map<string, number>();
  const binWithScore = new Map();
  const best = bs.best();
  if (!best) {
    return binWithScore;
  }
  const bestPrice = best?.price;

  const binStart =
    bestPrice +
    (((direction == 'bids' ? -1 : 1) * range.start) / 10000) * bestPrice;
  const binEnd =
    bestPrice +
    (((direction == 'bids' ? -1 : 1) * range.end) / 10000) * bestPrice;

  let lastSeenPrice = best.price;
  let queuePosition = 0; // TODO unused
  for (const item of bs.items()) {
    if (lastSeenPrice != item.price) {
      lastSeenPrice = item.price;
      queuePosition = 0;
    } else {
      queuePosition = queuePosition + 1;
    }

    if (direction == 'bids' ? item.price <= binEnd : item.price >= binEnd) {
      break;
    }
    if (direction == 'bids' ? item.price > binStart : item.price < binStart) {
      continue;
    }
    bin.set(
      item.owner.toBase58(),
      (bin.get(item.owner.toBase58()) ?? 0) + item.size,
    );
  }

  for (const key of bin.keys()) {
    const size = bin.get(key);
    if (!size) {
      continue;
    }
    binWithScore.set(key, {
      size,
      score: size * range.scoreMultiplier,
    });
  }

  return binWithScore;
}

function doSide(bs: BookSide, direction: 'bids' | 'asks'): Map<string, number> {
  const bins: Map<string, { size: number; score: number }>[] = [];
  for (const range of [
    // Range end is exclusive, and start in inclusive
    { start: 0, end: 1, scoreMultiplier: 100, queueMultiplier: 1.25 },
    { start: 1, end: 5, scoreMultiplier: 50, queueMultiplier: 1.25 },
    { start: 5, end: 10, scoreMultiplier: 20, queueMultiplier: 1.25 },
    { start: 10, end: 20, scoreMultiplier: 7.5, queueMultiplier: 1.25 },
    { start: 20, end: 50, scoreMultiplier: 5, queueMultiplier: 1.25 },
    { start: 50, end: 100, scoreMultiplier: 2.5, queueMultiplier: 1.25 },
  ]) {
    bins.push(doBin(bs, direction, range));
  }

  const aggr = new Map();
  for (const bin of bins) {
    for (const accountPk of bin.keys()) {
      const value = bin.get(accountPk);
      if (!value) {
        continue;
      }
      const binScore = value.score;
      aggr.set(accountPk, (aggr.get(accountPk) ?? 0) + binScore);
    }
  }
  return aggr;
}

async function doMarket(
  client: MangoClient,
  pm: PerpMarket,
): Promise<Map<string, number>> {
  const bidsAggr = doSide(await pm.loadBids(client, true), 'bids');
  const asksAggr = doSide(await pm.loadAsks(client, true), 'asks');

  const marketAggr = new Map();
  const marketAggrNorm = new Map();

  for (const accountPk of bidsAggr.keys()) {
    const score = bidsAggr.get(accountPk);
    if (!score) {
      continue;
    }
    marketAggr.set(accountPk, (marketAggr.get(accountPk) ?? 0) + score);
  }

  for (const accountPk of asksAggr.keys()) {
    const score = asksAggr.get(accountPk);
    if (!score) {
      continue;
    }
    marketAggr.set(accountPk, (marketAggr.get(accountPk) ?? 0) + score);
  }

  const scoreSum = Array.from(marketAggr.values()).reduce((a, b) => a + b, 0);
  for (const key of marketAggr.keys()) {
    marketAggrNorm.set(key, (marketAggr.get(key) / scoreSum) * 100);
  }

  return marketAggrNorm;
}

async function main(): Promise<void> {
  const client = await buildClient();
  const group = await client.getGroup(new PublicKey(GROUP_PK));

  for (const pm of group.perpMarketsMapByMarketIndex.values()) {
    console.log(pm.name, await doMarket(client, pm));
  }
}

main();
