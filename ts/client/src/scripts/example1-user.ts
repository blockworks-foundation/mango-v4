import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { HealthType } from '../accounts/mangoAccount';
import { OrderType, Side } from '../accounts/perp';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { toUiDecimalsForQuote } from '../utils';

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

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

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
  const client = await MangoClient.connect(
    userProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  // const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  const group = await client.getGroup(
    new PublicKey('FdynL6q7CNJMMiTZpfnYVkqQRYaoiBWgWkFYvvpx9uA8'),
  );
  console.log(group.toString());

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  if (true) {
    // set delegate, and change name
    console.log(`...changing mango account name, and setting a delegate`);
    const randomKey = new PublicKey(
      '4ZkS7ZZkxfsC3GtvvsHP3DFcUeByU9zzZELS4r8HCELo',
    );

    await client.editMangoAccount(
      group,
      mangoAccount,
      'my_changed_name',
      randomKey,
    );
    await mangoAccount.reload(client, group);
    console.log(mangoAccount.toString());

    console.log(`...resetting mango account name, and re-setting a delegate`);
    await client.editMangoAccount(
      group,
      mangoAccount,
      'my_mango_account',
      PublicKey.default,
    );
    await mangoAccount.reload(client, group);
    console.log(mangoAccount.toString());
  }

  if (true) {
    console.log(
      `...expanding mango account to have serum3 and perp position slots`,
    );
    await client.expandMangoAccount(group, mangoAccount, 16, 8, 8, 8);
    await mangoAccount.reload(client, group);
  }

  if (true) {
    // deposit and withdraw

    try {
      console.log(`...depositing 50 USDC`);
      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS['USDC']),
        50,
      );
      await mangoAccount.reload(client, group);

      console.log(`...withdrawing 1 USDC`);
      await client.tokenWithdraw(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS['USDC']),
        1,
        true,
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
    } catch (error) {
      console.log(error);
    }

    // witdrawing fails if no (other) user has deposited ORCA in the group
    // console.log(`Withdrawing...0.1 ORCA`);
    // await client.tokenWithdraw2(
    //   group,
    //   mangoAccount,
    //   'ORCA',
    //   0.1 * Math.pow(10, group.banksMap.get('ORCA').mintDecimals),
    //   true,
    // );
    // await mangoAccount.reload(client, group);
    // console.log(mangoAccount.toString());

    // serum3
    console.log(
      `...placing serum3 bid which would not be settled since its relatively low then midprice`,
    );
    await client.serum3PlaceOrder(
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
    await mangoAccount.reload(client, group);

    console.log(`...placing serum3 bid way above midprice`);
    await client.serum3PlaceOrder(
      group,
      mangoAccount,

      'BTC/USDC',
      Serum3Side.bid,
      90000,
      0.0001,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );
    await mangoAccount.reload(client, group);

    console.log(`...placing serum3 ask way below midprice`);
    await client.serum3PlaceOrder(
      group,
      mangoAccount,

      'BTC/USDC',
      Serum3Side.ask,
      30000,
      0.0001,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );

    console.log(`...current own orders on OB`);
    let orders = await client.getSerum3Orders(
      group,

      'BTC/USDC',
    );
    for (const order of orders) {
      console.log(
        `  - order orderId ${order.orderId}, ${order.side}, ${order.price}, ${order.size}`,
      );
      console.log(`  - cancelling order with ${order.orderId}`);
      await client.serum3CancelOrder(
        group,
        mangoAccount,

        'BTC/USDC',
        order.side === 'buy' ? Serum3Side.bid : Serum3Side.ask,
        order.orderId,
      );
    }

    console.log(`...current own orders on OB`);
    orders = await client.getSerum3Orders(
      group,

      'BTC/USDC',
    );
    for (const order of orders) {
      console.log(order);
    }

    console.log(`...settling funds`);
    await client.serum3SettleFunds(
      group,
      mangoAccount,

      'BTC/USDC',
    );
  }

  if (true) {
    await mangoAccount.reload(client, group);
    console.log(
      '...mangoAccount.getEquity() ' +
        toUiDecimalsForQuote(mangoAccount.getEquity().toNumber()),
    );
    console.log(
      '...mangoAccount.getCollateralValue() ' +
        toUiDecimalsForQuote(mangoAccount.getCollateralValue().toNumber()),
    );
    console.log(
      '...mangoAccount.accountData["healthCache"].health(HealthType.init) ' +
        toUiDecimalsForQuote(
          mangoAccount.accountData['healthCache']
            .health(HealthType.init)
            .toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getAssetsVal() ' +
        toUiDecimalsForQuote(
          mangoAccount.getAssetsVal(HealthType.init).toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getLiabsVal() ' +
        toUiDecimalsForQuote(
          mangoAccount.getLiabsVal(HealthType.init).toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getMaxWithdrawWithBorrowForToken(group, "SOL") ' +
        toUiDecimalsForQuote(
          (
            await mangoAccount.getMaxWithdrawWithBorrowForToken(
              group,
              new PublicKey(DEVNET_MINTS['SOL']),
            )
          ).toNumber(),
        ),
    );
    console.log(
      "...mangoAccount.getSerum3MarketMarginAvailable(group, 'BTC/USDC') " +
        toUiDecimalsForQuote(
          mangoAccount
            .getSerum3MarketMarginAvailable(group, 'BTC/USDC')
            .toNumber(),
        ),
    );
    console.log(
      "...mangoAccount.getPerpMarketMarginAvailable(group, 'BTC-PERP') " +
        toUiDecimalsForQuote(
          mangoAccount
            .getPerpMarketMarginAvailable(group, 'BTC-PERP')
            .toNumber(),
        ),
    );
  }

  if (true) {
    // perps
    console.log(`...placing perp bid`);
    try {
      await client.perpPlaceOrder(
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

    console.log(`...placing perp ask`);
    await client.perpPlaceOrder(
      group,
      mangoAccount,
      'BTC-PERP',
      Side.ask,
      30000,
      0.000001,
      30000 * 0.000001,
      Math.floor(Math.random() * 99999),
      OrderType.limit,
      0,
      1,
    );

    while (true) {
      // TODO: quotePositionNative might be buggy on program side, investigate...
      console.log(
        `...waiting for self trade to consume (note: make sure keeper crank is running)`,
      );
      await mangoAccount.reload(client, group);
      console.log(mangoAccount.toString());
    }
  }

  process.exit();
}

main();
