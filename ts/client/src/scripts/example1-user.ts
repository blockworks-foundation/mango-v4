import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { OrderType, Side } from '../accounts/perp';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../accounts/serum3';
import { MangoClient } from '../client';
import { DEVNET_SERUM3_PROGRAM_ID } from '../constants';

//
// An example for users based on high level api i.e. the client
// Create
// process.env.USER_KEYPAIR - mango account owner keypair path
// process.env.ADMIN_KEYPAIR - group admin keypair path (useful for automatically finding the group)
//
async function main() {
  const options = AnchorProvider.defaultOptions();
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
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(userProvider, true);
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForAdmin(admin.publicKey);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
    0,
    'my_mango_account',
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);

  // deposit and withdraw
  console.log(`Depositing...5000000 USDC`);
  await client.deposit(group, mangoAccount, 'USDC', 50_000000);
  await mangoAccount.reload(client);

  console.log(`Depositing...5000000 BTC`);
  await client.deposit(group, mangoAccount, 'BTC', 5000000);
  await mangoAccount.reload(client);

  console.log(`Withdrawing...1000000 USDC`);
  await client.withdraw(group, mangoAccount, 'USDC', 1_000000, false);
  await mangoAccount.reload(client);

  // serum3
  console.log(
    `Placing serum3 bid which would not be settled since its relatively low then midprice...`,
  );
  await client.serum3PlaceOrder(
    group,
    mangoAccount,
    DEVNET_SERUM3_PROGRAM_ID,
    'BTC/USDC',
    Serum3Side.bid,
    20,
    0.0001,
    Serum3SelfTradeBehavior.decrementTake,
    Serum3OrderType.limit,
    Date.now(),
    10,
  );
  await mangoAccount.reload(client);

  console.log(`Placing serum3 bid way above midprice...`);
  await client.serum3PlaceOrder(
    group,
    mangoAccount,
    DEVNET_SERUM3_PROGRAM_ID,
    'BTC/USDC',
    Serum3Side.bid,
    90000,
    0.0001,
    Serum3SelfTradeBehavior.decrementTake,
    Serum3OrderType.limit,
    Date.now(),
    10,
  );
  await mangoAccount.reload(client);

  console.log(`Placing serum3 ask way below midprice...`);
  await client.serum3PlaceOrder(
    group,
    mangoAccount,
    DEVNET_SERUM3_PROGRAM_ID,
    'BTC/USDC',
    Serum3Side.ask,
    30000,
    0.0001,
    Serum3SelfTradeBehavior.decrementTake,
    Serum3OrderType.limit,
    Date.now(),
    10,
  );

  console.log(`Current own orders on OB...`);
  let orders = await client.getSerum3Orders(
    group,
    DEVNET_SERUM3_PROGRAM_ID,
    'BTC/USDC',
  );
  for (const order of orders) {
    console.log(
      ` - Order orderId ${order.orderId}, ${order.side}, ${order.price}, ${order.size}`,
    );
    console.log(` - Cancelling order with ${order.orderId}`);
    await client.serum3CancelOrder(
      group,
      mangoAccount,
      DEVNET_SERUM3_PROGRAM_ID,
      'BTC/USDC',
      order.side === 'buy' ? Serum3Side.bid : Serum3Side.ask,
      order.orderId,
    );
  }

  console.log(`Current own orders on OB...`);
  orders = await client.getSerum3Orders(
    group,
    DEVNET_SERUM3_PROGRAM_ID,
    'BTC/USDC',
  );
  for (const order of orders) {
    console.log(order);
  }

  // console.log(`Close mango account...`);
  // await client.closeMangoAccount(mangoAccount);

  console.log(`Settling funds...`);
  await client.serum3SettleFunds(
    group,
    mangoAccount,
    DEVNET_SERUM3_PROGRAM_ID,
    'BTC/USDC',
  );

  // perps
  console.log(`Placing perp bid...`);
  await client.perpPlaceOrder(
    group,
    mangoAccount,
    'BTC/USDC',
    Side.bid,
    1,
    1,
    65535,
    65535,
    OrderType.limit,
    0,
    1,
  );

  console.log(`Placing perp ask...`);
  await client.perpPlaceOrder(
    group,
    mangoAccount,
    'BTC/USDC',
    Side.ask,
    1,
    1,
    65535,
    65535,
    OrderType.limit,
    0,
    1,
  );

  while (true) {
    // TODO: quotePositionNative might be buggy on program side, investigate...
    console.log(
      `Waiting for self trade to consume (note: make sure keeper crank is running)...`,
    );
    await mangoAccount.reload(client);
    console.log(mangoAccount.toString());
  }

  process.exit();
}

main();
