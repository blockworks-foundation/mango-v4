import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
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
    const client = await MangoClient.connect(
      userProvider,
      'mainnet-beta',
      MANGO_V4_ID['mainnet-beta'],
      {
        idsSource: 'get-program-accounts',
      },
    );
    const group = await client.getGroup(
      new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
    );
    console.log(
      await client.getMangoAccountForOwner(
        group,
        new PublicKey('v3mmtZ8JjXkaAbRRMBiNsjJF1rnN3qsMQqRLMk7Nz2C'),
        3,
      ),
    );
    console.log(
      await client.getMangoAccountsForDelegate(
        group,
        new PublicKey('5P9rHX22jb3MDq46VgeaHZ2TxQDKezPxsxNX3MaXyHwT'),
      ),
    );

    // const admin = Keypair.fromSecretKey(
    //   Buffer.from(
    //     JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    //   ),
    // );
    // const group = await client.getGroupForCreator(admin.publicKey, 23);
    // const mangoAccount = (await client.getMangoAccountForOwner(
    //   group,
    //   user.publicKey,
    //   0,
    // )) as MangoAccount;
    // console.log(mangoAccount);

    // let sig = await client.accountExpandV2(
    //   group,
    //   mangoAccount,
    //   16,
    //   8,
    //   8,
    //   32,
    //   8,
    // );
    // console.log(sig);
    // mangoAccount = await client.getOrCreateMangoAccount(group);

    // let sig = await client.tokenConditionalSwapCreate(
    //   group,
    //   mangoAccount,
    //   0 as TokenIndex,
    //   1 as TokenIndex,
    //   0,
    //   73,
    //   81,
    //   TokenConditionalSwapPriceThresholdType.priceOverThreshold,
    //   99,
    //   101,
    //   true,
    //   true,
    // );
    // console.log(sig);
  } catch (error) {
    console.log(error);
  }
}

main();
