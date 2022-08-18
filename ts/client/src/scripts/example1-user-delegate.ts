import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { OrderType, Side } from '../accounts/perp';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// An example for users based on high level api i.e. the client
// Create
// process.env.USER_KEYPAIR - mango account owner keypair path
// process.env.ADMIN_KEYPAIR - group admin keypair path (useful for automatically finding the group)
//
// This script deposits some tokens, places some serum orders, cancels them, places some perp orders
//

const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['ORCA', 'orcarKHSqC5CDDsGbho8GKvwExejWHxTqGzXgcewB9L'],
  ['MNGO', 'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC'],
]);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  // mango account owner
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // delegate
  const delegate = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER3_KEYPAIR!, 'utf-8')),
    ),
  );
  const delegateWallet = new Wallet(delegate);
  const delegateProvider = new AnchorProvider(
    connection,
    delegateWallet,
    options,
  );
  // Note: simply create a client with delegate and use this client to execute ixs
  const delegateClient = await MangoClient.connect(
    delegateProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`Delegate ${delegateWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await delegateClient.getGroupForAdmin(admin.publicKey, 0);
  console.log(group.toString());

  // fetch mango account using owners pubkey
  console.log(`Fetching mangoaccount...`);
  const mangoAccount = (
    await delegateClient.getMangoAccountForOwner(group, user.publicKey)
  )[0];
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  if (true) {
    // set delegate, and change name
    console.log(`...changing mango account name, and setting a delegate`);
    await client.editMangoAccount(
      group,
      mangoAccount,
      'my_changed_name',
      delegate.publicKey,
    );
    await mangoAccount.reload(client, group);
    console.log(mangoAccount.toString());
  }

  if (true) {
    // deposit
    console.log(`...depositing 50 USDC`);
    await client.tokenDeposit(
      group,
      mangoAccount,
      new PublicKey(DEVNET_MINTS['USDC']),
      50,
    );
    await mangoAccount.reload(client, group);

    console.log(`...depositing 0.0005 BTC`);
    await client.tokenDeposit(
      group,
      mangoAccount,
      new PublicKey(DEVNET_MINTS['BTC']),
      0.0005,
    );
    await mangoAccount.reload(client, group);

    // serum3
    console.log(`...placing serum3 bid`);
    await delegateClient.serum3PlaceOrder(
      group,
      mangoAccount,
      'BTC/USDC',
      Serum3Side.bid,
      20,
      0.0001,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );
    await mangoAccount.reload(delegateClient, group);

    console.log(`...current own orders on OB`);
    let orders = await delegateClient.getSerum3Orders(
      group,

      'BTC/USDC',
    );
    for (const order of orders) {
      console.log(
        `  - order orderId ${order.orderId}, ${order.side}, ${order.price}, ${order.size}`,
      );
      console.log(`  - cancelling order with ${order.orderId}`);
      await delegateClient.serum3CancelOrder(
        group,
        mangoAccount,
        'BTC/USDC',
        order.side === 'buy' ? Serum3Side.bid : Serum3Side.ask,
        order.orderId,
      );
    }

    console.log(`...settling funds`);
    await delegateClient.serum3SettleFunds(
      group,
      mangoAccount,

      'BTC/USDC',
    );
  }

  if (true) {
    // perps
    console.log(`...placing perp bid`);
    try {
      await delegateClient.perpPlaceOrder(
        group,
        mangoAccount,
        'BTC-PERP',
        Side.bid,
        30000,
        0.000001,
        30000 * 0.000001,
        Math.floor(Math.random() * 99999),
        OrderType.limit,
        0,
        1,
      );
    } catch (error) {
      console.log(error);
    }
  }

  process.exit();
}

main();
