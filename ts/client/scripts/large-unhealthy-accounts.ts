import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { HealthType, MangoAccount } from '../src/accounts/mangoAccount';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { toUiDecimalsForQuote } from '../src/utils';

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
  const mangoAccountsWithHealth = (
    await client.getAllMangoAccounts(group, true)
  )
    .map((a: MangoAccount) => {
      return {
        account: a,
        healthRatio: a.getHealthRatioUi(group, HealthType.maint),
        equity: toUiDecimalsForQuote(a.getEquity(group)),
      };
    })
    .filter((a) => a.equity > 1000)
    .filter((a) => a.healthRatio < 50)
    .sort((a, b) => a.healthRatio - b.healthRatio);

  console.log(
    `${'Account'.padStart(45)}, ${'Health Ratio'.padStart(
      10,
    )}, ${'Equity'.padStart(10)}`,
  );
  for (const obj of mangoAccountsWithHealth) {
    console.log(
      `${obj.account.publicKey.toBase58().padStart(45)}: ${obj.healthRatio
        .toFixed(2)
        .padStart(8)} %, ${obj.equity.toLocaleString().padStart(10)} $`,
    );
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
