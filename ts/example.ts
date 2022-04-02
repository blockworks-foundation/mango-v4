import { Provider, Wallet, web3 } from '@project-serum/anchor';
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
  getSerum3MarketForBaseAndQuote,
  registerToken,
  serum3RegisterMarket,
  withdraw,
} from './instructions';
import { findOrCreate } from './utils';
import { Bank, Group, MangoAccount, Serum3Market } from './types';

async function registerBank(
  client: MangoClient,
  group: any,
  admin: Keypair,
  mint: PublicKey,
  oracle: PublicKey,
  payer: Keypair,
  tokenIndex: number,
): Promise<Bank> {
  return await findOrCreate<Bank>(
    'group',
    getBankForGroupAndMint,
    [client, group.publicKey, mint],
    registerToken,
    [client, group.publicKey, admin.publicKey, mint, oracle, payer, tokenIndex],
  );
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
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new Provider(connection, adminWallet, options);
  const adminClient = await MangoClient.connect(adminProvider, true);

  const payer = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  console.log(`Payer ${payer.publicKey.toBase58()}`);
  //
  // Find existing or create a new group
  //
  const group: Group = await findOrCreate(
    'group',
    getGroupForAdmin,
    [adminClient, admin.publicKey],
    createGroup,
    [adminClient, admin.publicKey, payer],
  );
  console.log(`Group ${group.publicKey}`);

  //
  // Find existing or register new tokens
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

  const btcBank = await findOrCreate<Bank>(
    'bank',
    getBankForGroupAndMint,
    [adminClient, group.publicKey, btcDevnetMint],
    registerToken,
    [
      adminClient,
      group.publicKey,
      admin.publicKey,
      btcDevnetMint,
      btcDevnetOracle,
      payer,
      0,
    ],
  );
  console.log(`BtcBank ${btcBank.publicKey}`);
  const usdcBank = await findOrCreate<Bank>(
    'bank',
    getBankForGroupAndMint,
    [adminClient, group.publicKey, usdcDevnetMint],
    registerToken,
    [
      adminClient,
      group.publicKey,
      admin.publicKey,
      usdcDevnetMint,
      usdtDevnetOracle,
      payer,
      0,
    ],
  );
  console.log(`UsdcBank ${usdcBank.publicKey}`);

  //
  // Find existing or register a new serum market
  //
  const serumProgramId = new web3.PublicKey(
    'DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY',
  );
  const serumMarketExternalPk = new web3.PublicKey(
    'DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB',
  );
  const serum3Market = await findOrCreate<Serum3Market>(
    'serum3Market',
    getSerum3MarketForBaseAndQuote,
    [adminClient, group.publicKey, btcBank.tokenIndex, usdcBank.tokenIndex],
    serum3RegisterMarket,
    [
      adminClient,
      group.publicKey,
      admin.publicKey,
      serumProgramId,
      serumMarketExternalPk,
      usdcBank.publicKey,
      btcBank.publicKey,
      payer,
      0,
    ],
  );
  console.log(`Serum3Market ${serum3Market.publicKey}`);

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
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  //
  // Create mango account
  //
  const mangoAccount = await findOrCreate<MangoAccount>(
    'mangoAccount',
    getMangoAccountsForGroupAndOwner,
    [userClient, group.publicKey, user.publicKey],
    createMangoAccount,
    [userClient, group.publicKey, user.publicKey, payer],
  );
  console.log(`MangoAccount ${serum3Market.publicKey}`);

  // deposit
  console.log(`Depositing...1000`);
  const btcTokenAccount = await spl.getAssociatedTokenAddress(
    btcDevnetMint,
    user.publicKey,
  );
  await deposit(
    userClient,
    group.publicKey,
    mangoAccount.publicKey,
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
    mangoAccount.publicKey,
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

  const freshAccount = await getMangoAccount(
    userClient,
    mangoAccount.publicKey,
  );
  console.log(
    `Mango account  ${freshAccount.getNativeDeposit(
      freshBank,
    )} Deposits for bank ${freshBank.tokenIndex}`,
  );

  // close mango account, note: close doesnt settle/withdraw for user atm,
  // only use when you want to free up a mango account address for testing on not-mainnet
  // await closeMangoAccount(userClient, account.publicKey, user.publicKey);
  // accounts = await getMangoAccountsForGroupAndOwner(
  //   userClient,
  //   group.publicKey,
  //   user.publicKey,
  // );
  // if (accounts.length === 0) {
  //   console.log(`Closed account ${account.publicKey}`);
  // }

  process.exit(0);
}

main();
