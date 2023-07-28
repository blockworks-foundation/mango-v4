import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoAccount } from '../../src/accounts/mangoAccount';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

async function main(): Promise<void> {
  try {
    const options = AnchorProvider.defaultOptions();
    const connection = new Connection(process.env.CLUSTER_URL!, options);

    const user = Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
      ),
    );
    const userWallet = new Wallet(user);
    const userProvider = new AnchorProvider(connection, userWallet, options);

    //
    // mainnet
    //

    // const client = await MangoClient.connect(
    //   userProvider,
    //   'mainnet-beta',
    //   MANGO_V4_ID['mainnet-beta'],
    //   {
    //     idsSource: 'get-program-accounts',
    //   },
    // );
    // const group = await client.getGroup(
    //   new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
    // );
    // console.log(
    //   await client.getMangoAccountForOwner(
    //     group,
    //     new PublicKey('v3mmtZ8JjXkaAbRRMBiNsjJF1rnN3qsMQqRLMk7Nz2C'),
    //     3,
    //   ),
    // );
    // console.log(
    //   await client.getMangoAccountsForDelegate(
    //     group,
    //     new PublicKey('5P9rHX22jb3MDq46VgeaHZ2TxQDKezPxsxNX3MaXyHwT'),
    //   ),
    // );

    //
    // devnet
    //

    const client = await MangoClient.connect(
      userProvider,
      'devnet',
      MANGO_V4_ID['devnet'],
      {
        idsSource: 'get-program-accounts',
      },
    );

    const admin = Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
      ),
    );
    const group = await client.getGroupForCreator(admin.publicKey, 37);
    let mangoAccount = (await client.getMangoAccountForOwner(
      group,
      user.publicKey,
      0,
    )) as MangoAccount;

    let sig;
    // let sig = await client.accountExpandV2(
    //   group,
    //   mangoAccount,
    //   16,
    //   8,
    //   8,
    //   32,
    //   8,
    // );
    console.log(sig);

    // sig = await client.tokenConditionalSwapStopLoss(
    //   group,
    //   mangoAccount,
    //   group.getFirstBankByTokenIndex(0 as TokenIndex).mint,
    //   group.getFirstBankByTokenIndex(1 as TokenIndex).mint,
    //   1,
    //   null,
    //   25,
    //   24,
    //   1,
    // );
    // console.log(sig);

    mangoAccount = (await client.getMangoAccountForOwner(
      group,
      user.publicKey,
      0,
    )) as MangoAccount;
    console.log(mangoAccount.tokenConditionalSwaps[2].toString(group));
  } catch (error) {
    console.log(error);
  }
}

main();
