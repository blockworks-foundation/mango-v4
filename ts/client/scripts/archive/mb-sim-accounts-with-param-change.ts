import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { HealthType } from '../../src/accounts/mangoAccount';
import { PerpMarketIndex } from '../../src/accounts/perp';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { I80F48 } from '../../src/numbers/I80F48';
import { toUiDecimalsForQuote } from '../../src/utils';

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const SOME_KEYPAIR =
  process.env.PAYER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  const someKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(SOME_KEYPAIR!, 'utf-8'))),
  );

  const someWallet = new Wallet(someKeypair);
  const someProvider = new AnchorProvider(connection, someWallet, options);
  const client = MangoClient.connect(
    someProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'api',
    },
  );

  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const btcPerpPerpMarket = group.perpMarketsMapByMarketIndex.get(
    0 as PerpMarketIndex,
  )!;

  // e.g. change btc-perp leverage to 5/10x
  btcPerpPerpMarket.maintBaseAssetWeight = I80F48.fromNumber(0.9);
  btcPerpPerpMarket.initBaseAssetWeight = I80F48.fromNumber(0.8);
  btcPerpPerpMarket.maintBaseLiabWeight = I80F48.fromNumber(1.1);
  btcPerpPerpMarket.initBaseLiabWeight = I80F48.fromNumber(1.2);

  const mangoAccountsWithHealth = (
    await client.getAllMangoAccounts(group, true)
  )
    .map((a) => {
      return { account: a, health: a.getHealth(group, HealthType.maint) };
    })
    .sort((a, b) => a.health.toNumber() - b.health.toNumber());

  for (const obj of mangoAccountsWithHealth) {
    console.log(
      `${obj.account.publicKey}: ${toUiDecimalsForQuote(obj.health).toFixed(
        2,
      )} `,
    );
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
