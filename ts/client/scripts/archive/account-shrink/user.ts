import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoAccount } from '../../../src/accounts/mangoAccount';
import { MangoClient } from '../../../src/client';
import { MANGO_V4_ID } from '../../../src/constants';

/* eslint-disable */

const GROUP_NUM = 2814;

async function main(): Promise<void> {
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
  const client = await MangoClient.connect(
    userProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
    {
      idsSource: 'get-program-accounts',
    },
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  const serumMarketExternal = Array.from(
    group.serum3MarketsMapByMarketIndex.values(),
  )[0]!.serumMarketExternal;
  const perpMarket1 = Array.from(group.perpMarketsMapByName.values())[0]!;
  const perpMarket2 = Array.from(group.perpMarketsMapByName.values())[1]!;
  const perpMarket3 = Array.from(group.perpMarketsMapByName.values())[2]!;

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  let mangoAccount = await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  );
  if (!mangoAccount) {
    await client.createMangoAccount(group, 0, 'some', 2, 1, 1, 1);
    mangoAccount = (await client.getMangoAccountForOwner(
      group,
      user.publicKey,
      0,
      true,
    )) as MangoAccount;
  }
  await mangoAccount!.reload(client);
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);

  let sig;

  console.log(`Expanding mangoaccount...`);
  sig = await client.expandMangoAccount(group, mangoAccount, 2, 1, 3, 3);
  console.log(`...expanded mangoAccount ${sig.signature}`);

  // tokens
  {
    sig = await client.tokenDeposit(
      group,
      mangoAccount,
      group.banksMapByName.get('USDC')![0].mint,
      50,
    );
    console.log(`...deposited usdc ${sig.signature}`);
    // await mangoAccount.reload(client);
    // // deposit SOL
    // sig = await client.tokenDeposit(
    //   group,
    //   mangoAccount,
    //   group.banksMapByName.get('SOL')![0].mint,
    //   1,
    // );
    // console.log(`...deposited sol ${sig.signature}`);
    // await mangoAccount.reload(client);
  }

  // serum3
  {
    // sig = await client.serum3PlaceOrder(
    //   group,
    //   mangoAccount,
    //   serumMarketExternal,
    //   Serum3Side.bid,
    //   1,
    //   1,
    //   Serum3SelfTradeBehavior.decrementTake,
    //   Serum3OrderType.limit,
    //   Date.now(),
    //   10,
    // );
    // console.log(`...placed serum3 order ${sig.signature}`);
    // await mangoAccount.reload(client);
    // sig = await client.serum3ConsumeEvents(group, serumMarketExternal);
    // console.log(`...consumed events ${sig.signature}`);
    // for (const _ of range(0, 10)) {
    //   sig = await client.serum3CancelAllOrders(
    //     group,
    //     mangoAccount,
    //     serumMarketExternal,
    //     100,
    //   );
    //   console.log(`...cancelled all serum3 oo ${sig.signature}`);
    //   sig = await client.serum3SettleFunds(
    //     group,
    //     mangoAccount,
    //     serumMarketExternal,
    //   );
    //   console.log(`...settled serum3 ${sig.signature}`);
    //   await mangoAccount.reload(client);
    //   if (
    //     mangoAccount
    //       .getSerum3OoAccount(
    //         group.getSerum3MarketByExternalMarket(serumMarketExternal)
    //           .marketIndex,
    //       )
    //       .freeSlotBits.zeroBits() === 0
    //   ) {
    //     break;
    //   }
    // }
    // sig = await client.serum3CloseOpenOrders(
    //   group,
    //   mangoAccount,
    //   serumMarketExternal,
    // );
    // console.log(`...closed serum3 oo ${sig.signature}`);
    // sig = await client.expandMangoAccount(group, mangoAccount, 2, 0, 3, 3);
    // console.log(`...resized mangoAccount ${sig.signature}`);
  }

  // perps
  {
    // sig = await client.perpCancelAllOrders(
    //   group,
    //   mangoAccount,
    //   perpMarket1.perpMarketIndex,
    //   10,
    // );
    // sig = await client.perpCancelAllOrders(
    //   group,
    //   mangoAccount,
    //   perpMarket2.perpMarketIndex,
    //   10,
    // );
    // sig = await client.perpCancelAllOrders(
    //   group,
    //   mangoAccount,
    //   perpMarket3.perpMarketIndex,
    //   10,
    // );
    // sig = await client.perpPlaceOrder(
    //   group,
    //   mangoAccount,
    //   perpMarket1.perpMarketIndex,
    //   PerpOrderSide.bid,
    //   1,
    //   1,
    // );
    // console.log(`...placed perp order ${sig.signature}`);
    // await mangoAccount.reload(client);
    // sig = await client.perpPlaceOrder(
    //   group,
    //   mangoAccount,
    //   perpMarket2.perpMarketIndex,
    //   PerpOrderSide.bid,
    //   1,
    //   1,
    // );
    // console.log(`...placed perp order ${sig.signature}`);
    // await mangoAccount.reload(client);
    // sig = await client.perpPlaceOrder(
    //   group,
    //   mangoAccount,
    //   perpMarket3.perpMarketIndex,
    //   PerpOrderSide.bid,
    //   1,
    //   1,
    // );
    // console.log(`...placed perp order ${sig.signature}`);
    // await mangoAccount.reload(client);
    // sig = await client.perpCancelAllOrders(
    //   group,
    //   mangoAccount,
    //   perpMarket1.perpMarketIndex,
    //   10,
    // );
    // console.log(`...perp cancel orders ${sig.signature}`);
    // sig = await client.perpDeactivatePosition(
    //   group,
    //   mangoAccount,
    //   perpMarket1.perpMarketIndex,
    // );
    // console.log(`...perp deactivate position ${sig.signature}`);
    // await mangoAccount.reload(client);
    // console.log(mangoAccount.toString(group));
    // console.log(`Resizing mangoaccount...`);
    // sig = await client.expandMangoAccount(group, mangoAccount, 2, 1, 2, 3);
    // console.log(`...resized mangoAccount ${sig.signature}`);
  }

  await mangoAccount.reload(client);
  console.log(mangoAccount.toString(group));

  process.exit();
}

main();
