import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// This script deposits some tokens, so other liquidation scripts can borrow.
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 1);
const ACCOUNT_NUM = Number(process.env.ACCOUNT_NUM || 0);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(admin);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    admin.publicKey,
    ACCOUNT_NUM,
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  // deposit
  try {
    console.log(`...depositing 10 USDC`);
    await client.tokenDeposit(group, mangoAccount, 'USDC', 10);
    await mangoAccount.reload(client, group);

    console.log(`...depositing 0.0004 BTC`);
    await client.tokenDeposit(group, mangoAccount, 'BTC', 0.0004);
    await mangoAccount.reload(client, group);

    console.log(`...depositing 0.25 SOL`);
    await client.tokenDeposit(group, mangoAccount, 'SOL', 0.25);
    await mangoAccount.reload(client, group);
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

main();
