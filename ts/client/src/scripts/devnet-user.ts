import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { HealthType } from '../accounts/mangoAccount';
import { BookSide, PerpOrderSide, PerpOrderType } from '../accounts/perp';
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
export const DEVNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', new PublicKey('DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB')],
  ['SOL/USDC', new PublicKey('5xWpt56U1NCuHoAEtpLeUrQcxDkEpNfScjfLFaRzLPgR')],
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
    {},
    'get-program-accounts',
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  let mangoAccount = (await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  ))!;
  if (!mangoAccount) {
    throw new Error(`MangoAccount not found for user ${user.publicKey}`);
  }
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString(group));

  await mangoAccount.reload(client);

  // set delegate, and change name
  if (true) {
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
    await mangoAccount.reload(client);
    console.log(mangoAccount.toString());

    console.log(`...resetting mango account name, and re-setting a delegate`);
    await client.editMangoAccount(
      group,
      mangoAccount,
      'my_mango_account',
      PublicKey.default,
    );
    await mangoAccount.reload(client);
    console.log(mangoAccount.toString());
  }

  // expand account
  if (false) {
    console.log(
      `...expanding mango account to have serum3 and perp position slots`,
    );
    await client.expandMangoAccount(group, mangoAccount, 8, 8, 8, 8);
    await mangoAccount.reload(client);
  }

  // deposit and withdraw
  if (true) {
    try {
      console.log(`...depositing 50 USDC, 1 SOL, 1 MNGO`);
      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('USDC')!),
        50,
      );
      await mangoAccount.reload(client);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('SOL')!),
        1,
      );
      await mangoAccount.reload(client);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('MNGO')!),
        1,
      );
      await mangoAccount.reload(client);

      console.log(`...withdrawing 1 USDC`);
      await client.tokenWithdraw(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('USDC')!),
        1,
        true,
      );
      await mangoAccount.reload(client);

      console.log(`...depositing 0.0005 BTC`);
      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('BTC')!),
        0.0005,
      );
      await mangoAccount.reload(client);

      console.log(mangoAccount.toString(group));
    } catch (error) {
      console.log(error);
    }
  }

  if (true) {
    // serum3
    const serum3Market = group.serum3MarketsMapByExternal.get(
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')?.toBase58()!,
    );
    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')?.toBase58()!,
    );
    const asks = await group.loadSerum3AsksForMarket(
      client,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    const lowestAsk = Array.from(asks!)[0];
    const bids = await group.loadSerum3BidsForMarket(
      client,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    const highestBid = Array.from(asks!)![0];

    let price = 20;
    let qty = 0.0001;
    console.log(
      `...placing serum3 bid which would not be settled since its relatively low then midprice at ${price} for ${qty}`,
    );
    await client.serum3PlaceOrder(
      group,
      mangoAccount,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
      Serum3Side.bid,
      price,
      qty,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );
    await mangoAccount.reload(client);

    price = lowestAsk.price + lowestAsk.price / 2;
    qty = 0.0001;
    console.log(
      `...placing serum3 bid way above midprice at ${price} for ${qty}`,
    );
    await client.serum3PlaceOrder(
      group,
      mangoAccount,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
      Serum3Side.bid,
      price,
      qty,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );
    await mangoAccount.reload(client);

    price = highestBid.price - highestBid.price / 2;
    qty = 0.0001;
    console.log(
      `...placing serum3 ask way below midprice at ${price} for ${qty}`,
    );
    await client.serum3PlaceOrder(
      group,
      mangoAccount,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
      Serum3Side.ask,
      price,
      qty,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );

    console.log(`...current own orders on OB`);
    let orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
      client,
      group,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    for (const order of orders) {
      console.log(
        `  - order orderId ${order.orderId}, ${order.side}, ${order.price}, ${order.size}`,
      );
      console.log(`  - cancelling order with ${order.orderId}`);
      await client.serum3CancelOrder(
        group,
        mangoAccount,
        DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
        order.side === 'buy' ? Serum3Side.bid : Serum3Side.ask,
        order.orderId,
      );
    }

    console.log(`...current own orders on OB`);
    orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
      client,
      group,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    for (const order of orders) {
      console.log(order);
    }

    console.log(`...settling funds`);
    await client.serum3SettleFunds(
      group,
      mangoAccount,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
  }

  if (true) {
    // serum3 market
    const serum3Market = group.serum3MarketsMapByExternal.get(
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!.toBase58(),
    );
    console.log(await serum3Market?.logOb(client, group));
  }

  if (true) {
    await mangoAccount.reload(client);
    console.log(
      '...mangoAccount.getEquity() ' +
        toUiDecimalsForQuote(mangoAccount.getEquity(group)!.toNumber()),
    );
    console.log(
      '...mangoAccount.getCollateralValue() ' +
        toUiDecimalsForQuote(
          mangoAccount.getCollateralValue(group)!.toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getAssetsVal() ' +
        toUiDecimalsForQuote(
          mangoAccount.getAssetsValue(group, HealthType.init)!.toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getLiabsVal() ' +
        toUiDecimalsForQuote(
          mangoAccount.getLiabsValue(group, HealthType.init)!.toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getMaxWithdrawWithBorrowForToken(group, "SOL") ' +
        toUiDecimalsForQuote(
          mangoAccount
            .getMaxWithdrawWithBorrowForToken(
              group,
              new PublicKey(DEVNET_MINTS.get('SOL')!),
            )!
            .toNumber(),
        ),
    );
  }

  if (true) {
    const asks = await group.loadSerum3AsksForMarket(
      client,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    const lowestAsk = Array.from(asks!)[0];
    const bids = await group.loadSerum3BidsForMarket(
      client,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    const highestBid = Array.from(asks!)![0];

    function getMaxSourceForTokenSwapWrapper(src, tgt) {
      // console.log();
      console.log(
        `getMaxSourceForTokenSwap ${src.padEnd(4)} ${tgt.padEnd(4)} ` +
          mangoAccount.getMaxSourceUiForTokenSwap(
            group,
            group.banksMapByName.get(src)![0].mint,
            group.banksMapByName.get(tgt)![0].mint,
            1,
          )!,
      );
    }
    for (const srcToken of Array.from(group.banksMapByName.keys())) {
      for (const tgtToken of Array.from(group.banksMapByName.keys())) {
        getMaxSourceForTokenSwapWrapper(srcToken, tgtToken);
      }
    }

    const maxQuoteForSerum3BidUi = mangoAccount.getMaxQuoteForSerum3BidUi(
      group,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    console.log(
      "...mangoAccount.getMaxQuoteForSerum3BidUi(group, 'BTC/USDC') " +
        maxQuoteForSerum3BidUi,
    );

    const maxBaseForSerum3AskUi = mangoAccount.getMaxBaseForSerum3AskUi(
      group,
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
    );
    console.log(
      "...mangoAccount.getMaxBaseForSerum3AskUi(group, 'BTC/USDC') " +
        maxBaseForSerum3AskUi,
    );

    console.log(
      `simHealthRatioWithSerum3BidUiChanges ${mangoAccount.simHealthRatioWithSerum3BidUiChanges(
        group,
        785,
        DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
      )}`,
    );
    console.log(
      `simHealthRatioWithSerum3AskUiChanges ${mangoAccount.simHealthRatioWithSerum3AskUiChanges(
        group,
        0.033,
        DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
      )}`,
    );
  }

  // perps
  if (true) {
    let sig;
    const orders = await mangoAccount.loadPerpOpenOrdersForMarket(
      client,
      group,
      'BTC-PERP',
    );
    for (const order of orders) {
      console.log(
        `Current order - ${order.uiPrice} ${order.uiSize} ${order.side}`,
      );
    }
    console.log(`...cancelling all perp orders`);
    sig = await client.perpCancelAllOrders(group, mangoAccount, 'BTC-PERP', 10);
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // scenario 1
    // bid max perp
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price =
        group.banksMapByName.get('BTC')![0].uiPrice! -
        Math.floor(Math.random() * 100);
      const quoteQty = mangoAccount.getMaxQuoteForPerpBidUi(
        group,
        'BTC-PERP',
        1,
      );
      const baseQty = quoteQty / price;
      console.log(
        `...placing max qty perp bid  clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        'BTC-PERP',
        PerpOrderSide.bid,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }
    console.log(`...cancelling all perp orders`);
    sig = await client.perpCancelAllOrders(group, mangoAccount, 'BTC-PERP', 10);
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // bid max perp + some
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price =
        group.banksMapByName.get('BTC')![0].uiPrice! -
        Math.floor(Math.random() * 100);
      const quoteQty =
        mangoAccount.getMaxQuoteForPerpBidUi(group, 'BTC-PERP', 1) * 1.02;
      const baseQty = quoteQty / price;
      console.log(
        `...placing max qty * 1.02 perp bid clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        'BTC-PERP',
        PerpOrderSide.bid,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log('Errored out as expected');
    }

    // bid max ask
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price =
        group.banksMapByName.get('BTC')![0].uiPrice! +
        Math.floor(Math.random() * 100);
      const baseQty = mangoAccount.getMaxBaseForPerpAskUi(group, 'BTC-PERP', 1);
      const quoteQty = baseQty * price;
      console.log(
        `...placing max qty perp ask clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        'BTC-PERP',
        PerpOrderSide.ask,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }

    // bid max ask + some
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price =
        group.banksMapByName.get('BTC')![0].uiPrice! +
        Math.floor(Math.random() * 100);
      const baseQty =
        mangoAccount.getMaxBaseForPerpAskUi(group, 'BTC-PERP', 1) * 1.02;
      const quoteQty = baseQty * price;
      console.log(
        `...placing max qty perp ask * 1.02 clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        'BTC-PERP',
        PerpOrderSide.ask,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log('Errored out as expected');
    }

    console.log(`...cancelling all perp orders`);
    sig = await client.perpCancelAllOrders(group, mangoAccount, 'BTC-PERP', 10);
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // // scenario 2
    // // make + take orders
    // try {
    //   const clientId = Math.floor(Math.random() * 99999);
    //   const price = group.banksMapByName.get('BTC')![0].uiPrice!;
    //   console.log(`...placing perp bid ${clientId} at ${price}`);
    //   const sig = await client.perpPlaceOrder(
    //     group,
    //     mangoAccount,
    //     'BTC-PERP',
    //    PerpOrderSide.bid,
    //     price,
    //     0.01,
    //     price * 0.01,
    //     clientId,
    //     PerpOrderType.limit,
    //     0, //Date.now() + 200,
    //     1,
    //   );
    //   console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    // } catch (error) {
    //   console.log(error);
    // }
    // try {
    //   const clientId = Math.floor(Math.random() * 99999);
    //   const price = group.banksMapByName.get('BTC')![0].uiPrice!;
    //   console.log(`...placing perp ask ${clientId} at ${price}`);
    //   const sig = await client.perpPlaceOrder(
    //     group,
    //     mangoAccount,
    //     'BTC-PERP',
    //    PerpOrderSide.ask,
    //     price,
    //     0.01,
    //     price * 0.011,
    //     clientId,
    //     PerpOrderType.limit,
    //     0, //Date.now() + 200,
    //     1,
    //   );
    //   console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    // } catch (error) {
    //   console.log(error);
    // }
    // // // should be able to cancel them : know bug
    // // console.log(`...cancelling all perp orders`);
    // // sig = await client.perpCancelAllOrders(group, mangoAccount, 'BTC-PERP', 10);
    // // console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    const perpMarket = group.perpMarketsMap.get('BTC-PERP')!;

    const bids: BookSide = await perpMarket?.loadBids(client)!;
    console.log(`bids - ${Array.from(bids.items())}`);
    const asks: BookSide = await perpMarket?.loadAsks(client)!;
    console.log(`asks - ${Array.from(asks.items())}`);

    await perpMarket?.loadEventQueue(client)!;
    const fr = perpMarket?.getCurrentFundingRate(
      await perpMarket.loadBids(client),
      await perpMarket.loadAsks(client),
    );
    console.log(`current funding rate per hour is ${fr}`);

    const eq = await perpMarket?.loadEventQueue(client)!;
    console.log(`raw events - ${eq.rawEvents}`);

    // sleep so that keeper can catch up
    await new Promise((r) => setTimeout(r, 2000));

    // make+take orders should have cancelled each other, and if keeper has already cranked, then should not appear in position
    await group.reloadAll(client);
    await mangoAccount.reload(client);
    console.log(`${mangoAccount.toString(group)}`);
  }

  process.exit();
}

main();
