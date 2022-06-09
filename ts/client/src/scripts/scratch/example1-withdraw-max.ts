import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../../client';

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  /// user1
  const user1 = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const user1Wallet = new Wallet(user1);
  const user1Provider = new AnchorProvider(connection, user1Wallet, options);
  const user1Client = await MangoClient.connect(user1Provider, true);
  console.log(`user1 ${user1Wallet.publicKey.toBase58()}`);

  /// fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await user1Client.getGroupForAdmin(admin.publicKey, 0);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  /// fetch user1 account
  const user1MangoAccount = await user1Client.getOrCreateMangoAccount(
    group,
    user1.publicKey,
    0,
    'my_mango_account',
  );

  console.log(`...created/found mangoAccount ${user1MangoAccount.publicKey}`);

  /// user1 deposits some btc, so user2 can borrow it
  let amount = 0.001;
  let token = 'BTC';

  // console.log(`Depositing...${amount} 'BTC'`);
  // await user1Client.deposit(group, user1MangoAccount, token, amount);
  // await user1MangoAccount.reload(user1Client);
  // console.log(`${user1MangoAccount.toString(group)}`);

  console.log('---');

  /// user2 deposits some collateral and borrows BTC

  const user2 = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const user2Wallet = new Wallet(user2);
  const user2Provider = new AnchorProvider(connection, user2Wallet, options);
  const user2Client = await MangoClient.connect(user2Provider, true);
  console.log(`user2 ${user2Wallet.publicKey.toBase58()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const user2MangoAccount = await user2Client.getOrCreateMangoAccount(
    group,
    user2.publicKey,
    0,
    'my_mango_account',
  );
  console.log(`...created/found mangoAccount ${user2MangoAccount.publicKey}`);

  // console.log(`Depositing...${300} 'USDC'`);
  // await user2Client.deposit(group, user2MangoAccount, 'USDC', 300);
  // await user2MangoAccount.reload(user2Client);
  // console.log(`${user2MangoAccount.toString(group)}`);

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
  await user2MangoAccount.reload(user2Client);
  console.log(`${user2MangoAccount.toString(group)}`);

  process.exit();
}

main();
