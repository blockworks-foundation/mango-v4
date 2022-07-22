import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../../client';
import { MANGO_V4_ID } from '../../constants';

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
  const group = await user1Client.getGroupForAdmin(admin.publicKey, 0);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const user1MangoAccount = await user1Client.getOrCreateMangoAccount(
    group,
    user1.publicKey,
    0,
    AccountSize.small,
    'my_mango_account',
  );

  console.log(`...mangoAccount1 ${user1MangoAccount.publicKey}`);

  /// user1 deposits some btc, so user2 can borrow it
  let amount = 0.001;
  let token = 'BTC';
  console.log(`Depositing...${amount} 'BTC'`);
  await user1Client.tokenDeposit(group, user1MangoAccount, token, amount);
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
    0,
    AccountSize.small,
    'my_mango_account',
  );
  console.log(`...mangoAccount2 ${user2MangoAccount.publicKey}`);

  /// user2 deposits some collateral and borrows BTC
  console.log(`Depositing...${300} 'USDC'`);
  await user2Client.tokenDeposit(group, user2MangoAccount, 'USDC', 300);
  await user2MangoAccount.reload(user2Client, group);
  console.log(`${user2MangoAccount.toString(group)}`);
  amount = amount / 10;
  while (true) {
    try {
      console.log(`Withdrawing...${amount} 'BTC'`);
      await user2Client.tokenWithdraw(
        group,
        user2MangoAccount,
        token,
        amount,
        true,
      );
    } catch (error) {
      console.log(error);
      break;
    }
  }
  await user2MangoAccount.reload(user2Client, group);
  console.log(`${user2MangoAccount.toString(group)}`);

  /// Reduce usdc price
  console.log(
    `Setting USDC price to 0.9, to reduce health contribution of USDC collateral for user`,
  );
  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  await client.stubOracleSet(group, group.banksMap.get('USDC')?.oracle!, 0.5);

  process.exit();
}

main();
