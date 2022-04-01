import { Provider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import os from 'os';
import { MangoClient } from './client';
import {
  closeMangoAccount,
  createGroup,
  createMangoAccount,
  deposit,
  getBank,
  getBankForGroupAndMint,
  getGroupForAdmin,
  getMangoAccount,
  getMangoAccountsForGroupAndOwner,
  registerToken,
  withdraw,
} from './instructions';

async function main() {
  //
  // Setup
  //
  const options = Provider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(process.env.KEYPAIR!, 'utf-8'))),
  );
  const adminWallet = new Wallet(admin);

  const payer = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );

  const provider = new Provider(connection, adminWallet, options);
  const client = await MangoClient.connect(provider, true);

  //
  // Find existing or create a new group
  //
  let groups = await getGroupForAdmin(client, admin.publicKey);
  let group;
  if (groups.length > 0) {
    group = groups[0];
    console.log(`Found group ${group.publicKey.toBase58()}`);
  } else {
    await createGroup(client, admin.publicKey, payer);
    let groups = await getGroupForAdmin(client, admin.publicKey);
    group = groups[0];
    console.log(`Created group ${group.publicKey.toBase58()}`);
  }

  //
  // Find existing or register a new token
  //
  const btcDevnetMint = new PublicKey(
    '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU',
  );
  const btcDevnetOracle = new PublicKey(
    'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J',
  );
  let banks = await getBankForGroupAndMint(
    client,
    group.publicKey,
    btcDevnetMint,
  );
  let bank;
  if (banks.length > 0) {
    bank = banks[0];
    console.log(`Found bank ${bank.publicKey.toBase58()}`);
  } else {
    await registerToken(
      client,
      group.publicKey,
      admin.publicKey,
      btcDevnetMint,
      btcDevnetOracle,
      payer,
    );
    banks = await getBankForGroupAndMint(
      client,
      group.publicKey,
      btcDevnetMint,
    );
    bank = banks[0];
    console.log(`Registered token ${bank.publicKey.toBase58()}`);
  }

  //
  // Create mango account
  //
  let accounts = await getMangoAccountsForGroupAndOwner(
    client,
    group.publicKey,
    admin.publicKey,
  );
  let account;
  if (accounts.length > 0) {
    account = accounts[0];
    console.log(`Found mango account ${account.publicKey.toBase58()}`);
  } else {
    await createMangoAccount(client, group.publicKey, admin.publicKey, payer);
    accounts = await getMangoAccountsForGroupAndOwner(
      client,
      group.publicKey,
      admin.publicKey,
    );
    account = accounts[0];
    console.log(`Created mango account ${account.publicKey.toBase58()}`);
  }

  // deposit
  console.log(`Depositing...1000`);
  await deposit(
    client,
    group.publicKey,
    account.publicKey,
    bank.publicKey,
    bank.vault,
    // BTC token account
    new PublicKey('CzCgYE7hcWSM6T5iN1ev5zNHbPmywAqZQkStefVcADAL'),
    btcDevnetOracle,
    admin.publicKey,
    1000,
  );

  // withdraw
  console.log(`Witdrawing...500`);
  await withdraw(
    client,
    group.publicKey,
    account.publicKey,
    bank.publicKey,
    bank.vault,
    // BTC token account
    new PublicKey('CzCgYE7hcWSM6T5iN1ev5zNHbPmywAqZQkStefVcADAL'),
    btcDevnetOracle,
    admin.publicKey,
    500,
    false,
  );

  // log
  const freshBank = await getBank(client, bank.publicKey);
  console.log(freshBank.toString());

  const freshAccount = await getMangoAccount(client, account.publicKey);
  console.log(
    `Mango account  ${freshAccount.getNativeDeposit(
      freshBank,
    )} Deposits for bank ${freshBank.tokenIndex}`,
  );

  // close mango account
  await closeMangoAccount(client, account.publicKey, admin.publicKey);
  accounts = await getMangoAccountsForGroupAndOwner(
    client,
    group.publicKey,
    admin.publicKey,
  );
  if (accounts.length === 0) {
    console.log(`Closed account ${account.publicKey}`);
  }

  process.exit(0);
}

main();
