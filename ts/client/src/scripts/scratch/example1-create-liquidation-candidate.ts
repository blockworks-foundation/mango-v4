import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { AccountSize, MangoClient } from '../../index';

import { MANGO_V4_ID } from '../../constants';

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  // user1
  const user1 = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const user1Wallet = new Wallet(user1);
  const user1Provider = new AnchorProvider(connection, user1Wallet, options);
  const user1Client = await MangoClient.connect(
    user1Provider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`user1 ${user1Wallet.publicKey.toBase58()}`);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await user1Client.getGroupForAdmin(admin.publicKey, GROUP_NUM);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const user1MangoAccount = await user1Client.getOrCreateMangoAccount(
    group,
    user1.publicKey,
    user1,
    0,
    AccountSize.small,
    'my_mango_account'
  );

  console.log(`...mangoAccount1 ${user1MangoAccount.publicKey}`);

  /// user1 deposits some btc, so user2 can borrow it
  let amount = 0.001;
  let token = 'BTC';
  console.log(`Depositing...${amount} 'BTC'`);
  await user1Client.tokenDeposit(group, user1MangoAccount, token, amount, user1);
  await user1MangoAccount.reload(user1Client, group);
  console.log(`${user1MangoAccount.toString(group)}`);

  // user 2
  const user2 = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const user2Wallet = new Wallet(user2);
  const user2Provider = new AnchorProvider(connection, user2Wallet, options);
  const user2Client = await MangoClient.connect(
    user2Provider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`user2 ${user2Wallet.publicKey.toBase58()}`);

  const user2MangoAccount = await user2Client.getOrCreateMangoAccount(
    group,
    user2.publicKey,
    user2,
    0,
    AccountSize.small,
    'my_mango_account',
  );
  console.log(`...mangoAccount2 ${user2MangoAccount.publicKey}`);

  /// Increase usdc price temporarily to allow lots of borrows
  console.log(
    `Setting USDC price to 1.5, to allow the user to borrow lots of btc`,
  );
  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  await client.stubOracleSet(group, group.banksMap.get('USDC')?.oracle!, 1.5);

  /// user2 deposits some collateral and borrows BTC
  amount = 1;
  console.log(`Depositing...${amount} 'USDC'`);
  await user2Client.tokenDeposit(group, user2MangoAccount, 'USDC', amount, user2);
  await user2MangoAccount.reload(user2Client, group);
  console.log(`${user2MangoAccount.toString(group)}`);

  const maxNative = await (
    await user2MangoAccount.getMaxWithdrawWithBorrowForToken(group, token)
  ).toNumber();
  amount = 0.9 * maxNative;
  console.log(`Withdrawing...${amount} native BTC'`);
  await user2Client.tokenWithdrawNative(
    group,
    user2MangoAccount,
    token,
    amount,
    true,
    user2
  );
  await user2MangoAccount.reload(user2Client, group);
  console.log(`${user2MangoAccount.toString(group)}`);

  /// Reduce usdc price to normal again
  console.log(
    `Setting USDC price back to 1.0, decreasing the user's collateral size`,
  );
  await client.stubOracleSet(group, group.banksMap.get('USDC')?.oracle!, 1.0);

  process.exit();
}

main();
