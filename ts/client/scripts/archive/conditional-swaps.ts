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
    account
      .tokenConditionalSwapsActive()
      .forEach((tcs) => console.log(tcs.toString(group)));

    await Promise.all(
      account.tokenConditionalSwaps.map((tcs, i) => {
        if (!tcs.isConfigured) {
          return Promise.resolve();
        }
        client.tokenConditionalSwapCancel(group, account, tcs.id);
      }),
    );

    // const sig = await client.tcsTakeProfitOnDeposit(
    //   group,
    //   account,
    //   group.getFirstBankByTokenIndex(4 as TokenIndex),
    //   group.getFirstBankByTokenIndex(0 as TokenIndex),
    //   group.getFirstBankByTokenIndex(4 as TokenIndex).uiPrice + 1,
    //   false,
    //   null,
    //   null,
    //   null,
    // );

    // const sig = await client.tcsStopLossOnDeposit(
    //   group,
    //   account,
    //   group.getFirstBankByTokenIndex(4 as TokenIndex),
    //   group.getFirstBankByTokenIndex(0 as TokenIndex),
    //   group.getFirstBankByTokenIndex(4 as TokenIndex).uiPrice - 1,
    //   false,
    //   null,
    //   null,
    //   null,
    // );

    // const sig = await client.tcsTakeProfitOnBorrow(
    //   group,
    //   account,
    //   group.getFirstBankByTokenIndex(0 as TokenIndex),
    //   group.getFirstBankByTokenIndex(4 as TokenIndex),
    //   group.getFirstBankByTokenIndex(4 as TokenIndex).uiPrice - 1,
    //   true,
    //   null,
    //   null,
    //   null,
    //   null,
    // );

    const sig = await client.tcsStopLossOnBorrow(
      group,
      account,
      group.getFirstBankByTokenIndex(0 as TokenIndex),
      group.getFirstBankByTokenIndex(4 as TokenIndex),
      group.getFirstBankByTokenIndex(4 as TokenIndex).uiPrice + 1,
      true,
      null,
      null,
      null,
      null,
    );

    console.log(sig);

    account = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK));
    console.log(account.tokenConditionalSwaps[0].toString(group));
  } catch (error) {
    console.log(error);
  }
}

main();
