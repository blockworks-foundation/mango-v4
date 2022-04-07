import { Provider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoAccount } from './accounts/types/mangoAccount';
import { MangoClient } from './client';

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

  const group = await client.getGroup(
    new PublicKey('6ACH752p6FsdLzuociVkmDwc3wJW8pcCoxZKfXJKfKcD'),
  );
  console.log(`Group ${group.publicKey}`);

  const banks = await client.getBanksForGroup(group);
  for (const bank of banks) {
    console.log(`Bank ${bank.tokenIndex} ${bank.publicKey}`);
  }

  let mangoAccounts: MangoAccount[] = [];
  let mangoAccount: MangoAccount;
  mangoAccounts = await client.getMangoAccount(group, user.publicKey);
  if (mangoAccounts.length === 0) {
    await client.createMangoAccount(group, 0);
    mangoAccounts = await client.getMangoAccount(group, user.publicKey);
  }
  mangoAccount = mangoAccounts[0];
  console.log(`MangoAccount ${mangoAccount.publicKey}`);

  console.log(`Depositing...1000`);
  await client.deposit(group, mangoAccount, banks[0], 1000);

  console.log(`Withdrawing...500`);
  await client.withdraw(group, mangoAccount, banks[0], 500, false);
}

main();
