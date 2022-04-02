import { Provider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as spl from '@solana/spl-token';
import fs from 'fs';
import { MangoClient } from './client';
import {
  closeMangoAccount,
  createGroup,
  createMangoAccount,
  deposit,
  getBank,
  getBankForGroupAndMint,
  getBanksForGroup,
  getGroupForAdmin,
  getMangoAccount,
  getMangoAccountsForGroupAndOwner,
  registerToken,
  withdraw,
} from './instructions';

async function registerBank(
  client: MangoClient,
  group: any,
  admin: Keypair,
  mint: PublicKey,
  oracle: PublicKey,
  payer: Keypair,
  tokenIndex: number,
) {
  let banks = await getBankForGroupAndMint(client, group.publicKey, mint);
  let bank;
  if (banks.length > 0) {
    bank = banks[0];
    console.log(`Found bank ${bank.publicKey.toBase58()}`);
  } else {
    await registerToken(
      client,
      group.publicKey,
      admin.publicKey,
      mint,
      oracle,
      payer,
      tokenIndex,
    );
    banks = await getBankForGroupAndMint(client, group.publicKey, mint);
    bank = banks[0];
    console.log(`Registered token ${bank.publicKey.toBase58()}`);
  }
  return bank;
}

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
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  console.log(`admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new Provider(connection, adminWallet, options);
  const adminClient = await MangoClient.connect(adminProvider, true);

  const payer = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  console.log(`payer ${payer.publicKey.toBase58()}`);
  //
  // Find existing or create a new group
  //
  let groups = await getGroupForAdmin(adminClient, admin.publicKey);
  let group;
  if (groups.length > 0) {
    group = groups[0];
    console.log(`Found group ${group.publicKey.toBase58()}`);
  } else {
    await createGroup(adminClient, admin.publicKey, payer);
    let groups = await getGroupForAdmin(adminClient, admin.publicKey);
    group = groups[0];
    console.log(`Created group ${group.publicKey.toBase58()}`);
  }

  //
  // Find existing or register a new token
  //
  // TODO: replace with 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU,
  // see https://developers.circle.com/docs/usdc-on-testnet#usdc-on-solana-testnet
  const usdcDevnetMint = new PublicKey(
    '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN',
  );
  // TODO: replace with a usdc devnet oracle
  const usdtDevnetOracle = new PublicKey(
    '38xoQ4oeJCBrcVvca2cGk7iV1dAfrmTR1kmhSCJQ8Jto',
  );
  const btcDevnetMint = new PublicKey(
    '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU',
  );
  const btcDevnetOracle = new PublicKey(
    'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J',
  );
  let btcBank = await registerBank(
    adminClient,
    group,
    admin,
    btcDevnetMint,
    btcDevnetOracle,
    payer,
    0,
  );
  let usdcBank = await registerBank(
    adminClient,
    group,
    admin,
    usdcDevnetMint,
    usdtDevnetOracle,
    payer,
    1,
  );

  //
  // User operations
  //

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new Provider(connection, userWallet, options);
  const userClient = await MangoClient.connect(userProvider, true);
  console.log(`user ${userWallet.publicKey.toBase58()}`);

  //
  // Create mango account
  //
  let accounts = await getMangoAccountsForGroupAndOwner(
    userClient,
    group.publicKey,
    user.publicKey,
  );
  let account;
  if (accounts.length > 0) {
    account = accounts[0];
    console.log(`Found mango account ${account.publicKey.toBase58()}`);
  } else {
    await createMangoAccount(
      userClient,
      group.publicKey,
      user.publicKey,
      payer,
    );
    accounts = await getMangoAccountsForGroupAndOwner(
      userClient,
      group.publicKey,
      user.publicKey,
    );
    account = accounts[0];
    console.log(`Created mango account ${account.publicKey.toBase58()}`);
  }

  // deposit
  console.log(`Depositing...1000`);
  const btcTokenAccount = await spl.getAssociatedTokenAddress(
    btcDevnetMint,
    user.publicKey,
  );
  await deposit(
    userClient,
    group.publicKey,
    account.publicKey,
    btcBank.publicKey,
    btcBank.vault,
    btcTokenAccount,
    btcDevnetOracle,
    user.publicKey,
    1000,
  );

  // withdraw
  console.log(`Witdrawing...500`);
  await withdraw(
    userClient,
    group.publicKey,
    account.publicKey,
    btcBank.publicKey,
    btcBank.vault,
    btcTokenAccount,
    btcDevnetOracle,
    user.publicKey,
    500,
    false,
  );

  // log
  const freshBank = await getBank(userClient, btcBank.publicKey);
  console.log(freshBank.toString());

  const freshAccount = await getMangoAccount(userClient, account.publicKey);
  console.log(
    `Mango account  ${freshAccount.getNativeDeposit(
      freshBank,
    )} Deposits for bank ${freshBank.tokenIndex}`,
  );

  // close mango account
  await closeMangoAccount(userClient, account.publicKey, user.publicKey);
  accounts = await getMangoAccountsForGroupAndOwner(
    userClient,
    group.publicKey,
    user.publicKey,
  );
  if (accounts.length === 0) {
    console.log(`Closed account ${account.publicKey}`);
  }

  process.exit(0);
}

main();
