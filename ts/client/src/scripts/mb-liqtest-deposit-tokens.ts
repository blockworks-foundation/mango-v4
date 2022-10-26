import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// This script deposits some tokens, so other liquidation scripts can borrow.
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 200);
const ACCOUNT_NUM = Number(process.env.ACCOUNT_NUM || 0);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL!, options);

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
    {
      idsSource: 'get-program-accounts',
    },
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = (await client.createAndFetchMangoAccount(
    group,
    ACCOUNT_NUM,
    'LIQTEST, FUNDING',
    8,
    4,
    4,
    4,
  ))!;
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  const usdcMint = group.banksMapByName.get('USDC')![0].mint;
  const btcMint = group.banksMapByName.get('BTC')![0].mint;
  const solMint = group.banksMapByName.get('SOL')![0].mint;

  // deposit
  try {
    console.log(`...depositing 5 USDC`);
    await client.tokenDeposit(group, mangoAccount, usdcMint, 5);
    await mangoAccount.reload(client, group);

    console.log(`...depositing 0.0002 BTC`);
    await client.tokenDeposit(group, mangoAccount, btcMint, 0.0002);
    await mangoAccount.reload(client, group);

    console.log(`...depositing 0.15 SOL`);
    await client.tokenDeposit(group, mangoAccount, solMint, 0.15);
    await mangoAccount.reload(client, group);
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

main();
