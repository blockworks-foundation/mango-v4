import { Provider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from './accounts/types/serum3';
import { MangoClient } from './client';
import { DEVNET_GROUP, DEVNET_SERUM3_PROGRAM_ID } from './constants';

//
// An example for users based on high level api i.e. the client
//
async function main() {
  const options = Provider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new Provider(connection, userWallet, options);
  const client = await MangoClient.connect(userProvider, true);
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroup(new PublicKey(DEVNET_GROUP));
  console.log(`Group ${group.publicKey.toBase58()}`);

  // create + fetch account
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
    0,
  );
  console.log(`MangoAccount ${mangoAccount.publicKey}`);

  // deposit and withdraw
  console.log(`Depositing...1000000`);
  await client.deposit(group, mangoAccount, 'USDC', 1000000);
  console.log(`Withdrawing...500000`);
  await client.withdraw(group, mangoAccount, 'USDC', 500000, false);

  // serum3
  console.log(`Placing serum3 order`);
  await client.serum3PlaceOrder(
    group,
    mangoAccount,
    DEVNET_SERUM3_PROGRAM_ID,
    'BTC/USDC',
    Serum3Side.bid,
    40000,
    1,
    1000000,
    Serum3SelfTradeBehavior.decrementTake,
    Serum3OrderType.limit,
    Date.now(),
    10,
  );

  process.exit();
}

main();
