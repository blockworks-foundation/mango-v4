import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../../src/accounts/bank';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;

async function main(): Promise<void> {
  try {
    const options = AnchorProvider.defaultOptions();
    const connection = new Connection(CLUSTER_URL!, options);

    const user = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(USER_KEYPAIR!, 'utf-8'))),
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

    const group = await client.getGroup(
      new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
    );

    let account = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK));
    await Promise.all(
      account.tokenConditionalSwaps.map((tcs, i) => {
        if (!tcs.hasData) {
          return Promise.resolve();
        }
        client.tokenConditionalSwapCancel(group, account, i, tcs.id);
      }),
    );

    await client.tokenConditionalSwapStopLoss(
      group,
      account,
      group.getFirstBankByTokenIndex(0 as TokenIndex).mint,
      group.getFirstBankByTokenIndex(6 as TokenIndex).mint,
      account.getTokenBalanceUi(
        group.getFirstBankByTokenIndex(6 as TokenIndex),
      ),
      null,
      group.getFirstBankByTokenIndex(6 as TokenIndex).uiPrice * 1.1,
      0,
      2,
    );

    account = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK));
    console.log(account.tokenConditionalSwaps[0].toString(group));
  } catch (error) {
    console.log(error);
  }
}

main();
