import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { expect } from 'chai';
import fs from 'fs';
import { Group } from '../../src/accounts/group';
import { MangoAccount } from '../../src/accounts/mangoAccount';
import { PerpOrderSide, PerpOrderType } from '../../src/accounts/perp';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { toUiDecimalsForQuote } from '../../src/utils';

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
]);
export const DEVNET_SERUM3_MARKETS = new Map([
  ['SOL/USDC', new PublicKey('6xYbSQyhajUqyatJDdkonpj7v41bKeEBWpf7kwRh5X7A')],
]);

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

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

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = (await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  )) as MangoAccount;
  await mangoAccount!.reload(client);
  if (!mangoAccount) {
    throw new Error(`MangoAccount not found for user ${user.publicKey}`);
  }
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);

  // set delegate, and change name
  // eslint-disable-next-line no-constant-condition
  if (true) {
    console.log(`...changing mango account name, and setting a delegate`);
    const newName = 'my_changed_name';
    const randomKey = new PublicKey(
      '4ZkS7ZZkxfsC3GtvvsHP3DFcUeByU9zzZELS4r8HCELo',
    );

    await client.editMangoAccount(group, mangoAccount, newName, randomKey);
    await mangoAccount.reload(client);
    expect(mangoAccount.name).deep.equals(newName);
    expect(mangoAccount.delegate).deep.equals(randomKey);

    const oldName = 'my_mango_account';
    console.log(`...resetting mango account name, and re-setting a delegate`);
    await client.editMangoAccount(
      group,
      mangoAccount,
      oldName,
      PublicKey.default,
    );
    await mangoAccount.reload(client);
    expect(mangoAccount.name).deep.equals(oldName);
    expect(mangoAccount.delegate).deep.equals(PublicKey.default);
  }

  // expand account
  if (
    mangoAccount.tokens.length < 16 ||
    mangoAccount.serum3.length < 8 ||
    mangoAccount.perps.length < 8 ||
    mangoAccount.perpOpenOrders.length < 8
  ) {
    console.log(
      `...expanding mango account to max 16 token positions, 8 serum3, 8 perp position and 8 perp oo slots, previous (tokens ${mangoAccount.tokens.length}, serum3 ${mangoAccount.serum3.length}, perps ${mangoAccount.perps.length}, perps oo ${mangoAccount.perpOpenOrders.length})`,
    );
    const sig = await client.expandMangoAccount(
      group,
      mangoAccount,
      16,
      8,
      8,
      8,
    );
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    await mangoAccount.reload(client);
    expect(mangoAccount.tokens.length).equals(16);
    expect(mangoAccount.serum3.length).equals(8);
    expect(mangoAccount.perps.length).equals(8);
    expect(mangoAccount.perpOpenOrders.length).equals(8);
  }

  // deposit and withdraw
  // eslint-disable-next-line no-constant-condition
  if (true) {
    console.log(`...depositing 50 USDC, 1 SOL, 1 MNGO`);

    // deposit USDC
    let oldBalance = mangoAccount.getTokenBalance(
      group.getFirstBankByMint(new PublicKey(DEVNET_MINTS.get('USDC')!)),
    );
    await client.tokenDeposit(
      group,
      mangoAccount,
      new PublicKey(DEVNET_MINTS.get('USDC')!),
      50,
    );
    await mangoAccount.reload(client);
    let newBalance = mangoAccount.getTokenBalance(
      group.getFirstBankByMint(new PublicKey(DEVNET_MINTS.get('USDC')!)),
    );
    expect(toUiDecimalsForQuote(newBalance.sub(oldBalance)).toString()).equals(
      '50',
    );

    // deposit SOL
    await client.tokenDeposit(
      group,
      mangoAccount,
      new PublicKey(DEVNET_MINTS.get('SOL')!),
      1,
    );
    await mangoAccount.reload(client);

    // deposit MNGO
    await client.tokenDeposit(
      group,
      mangoAccount,
      new PublicKey(DEVNET_MINTS.get('MNGO')!),
      1,
    );
    await mangoAccount.reload(client);

    // withdraw USDC
    console.log(`...withdrawing 1 USDC`);
    oldBalance = mangoAccount.getTokenBalance(
      group.getFirstBankByMint(new PublicKey(DEVNET_MINTS.get('USDC')!)),
    );
    await client.tokenWithdraw(
      group,
      mangoAccount,
      new PublicKey(DEVNET_MINTS.get('USDC')!),
      1,
      true,
    );
    await mangoAccount.reload(client);
    newBalance = mangoAccount.getTokenBalance(
      group.getFirstBankByMint(new PublicKey(DEVNET_MINTS.get('USDC')!)),
    );
    expect(toUiDecimalsForQuote(oldBalance.sub(newBalance)).toString()).equals(
      '1',
    );

    console.log(`...depositing 0.0005 BTC`);
    await client.tokenDeposit(
      group,
      mangoAccount,
      new PublicKey(DEVNET_MINTS.get('BTC')!),
      0.0005,
    );
    await mangoAccount.reload(client);
  }

  // Note: Disable for now until we have openbook devnet markets
  // if (true) {
  //   // serum3
  //   const asks = await group.loadSerum3AsksForMarket(
  //     client,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //   );
  //   const lowestAsk = Array.from(asks!)[0];
  //   const bids = await group.loadSerum3BidsForMarket(
  //     client,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //   );
  //   const highestBid = Array.from(bids!)![0];

  //   console.log(`...cancelling all existing serum3 orders`);
  //   if (
  //     Array.from(mangoAccount.serum3OosMapByMarketIndex.values()).length > 0
  //   ) {
  //     await client.serum3CancelAllOrders(
  //       group,
  //       mangoAccount,
  //       DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //       10,
  //     );
  //   }

  //   let price = 20;
  //   let qty = 0.0001;
  //   console.log(
  //     `...placing serum3 bid which would not be settled since its relatively low then midprice at ${price} for ${qty}`,
  //   );
  //   await client.serum3PlaceOrder(
  //     group,
  //     mangoAccount,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //     Serum3Side.bid,
  //     price,
  //     qty,
  //     Serum3SelfTradeBehavior.decrementTake,
  //     Serum3OrderType.limit,
  //     Date.now(),
  //     10,
  //   );
  //   await mangoAccount.reload(client);
  //   let orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
  //     client,
  //     group,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //   );
  //   expect(orders[0].price).equals(20);
  //   expect(orders[0].size).equals(qty);

  //   price = lowestAsk.price + lowestAsk.price / 2;
  //   qty = 0.0001;
  //   console.log(
  //     `...placing serum3 bid way above midprice at ${price} for ${qty}`,
  //   );
  //   await client.serum3PlaceOrder(
  //     group,
  //     mangoAccount,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //     Serum3Side.bid,
  //     price,
  //     qty,
  //     Serum3SelfTradeBehavior.decrementTake,
  //     Serum3OrderType.limit,
  //     Date.now(),
  //     10,
  //   );
  //   await mangoAccount.reload(client);

  //   price = highestBid.price - highestBid.price / 2;
  //   qty = 0.0001;
  //   console.log(
  //     `...placing serum3 ask way below midprice at ${price} for ${qty}`,
  //   );
  //   await client.serum3PlaceOrder(
  //     group,
  //     mangoAccount,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //     Serum3Side.ask,
  //     price,
  //     qty,
  //     Serum3SelfTradeBehavior.decrementTake,
  //     Serum3OrderType.limit,
  //     Date.now(),
  //     10,
  //   );

  //   console.log(`...current own orders on OB`);
  //   orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
  //     client,
  //     group,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //   );
  //   for (const order of orders) {
  //     console.log(
  //       `  - order orderId ${order.orderId}, ${order.side}, ${order.price}, ${order.size}`,
  //     );
  //     console.log(`  - cancelling order with ${order.orderId}`);
  //     await client.serum3CancelOrder(
  //       group,
  //       mangoAccount,
  //       DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //       order.side === 'buy' ? Serum3Side.bid : Serum3Side.ask,
  //       order.orderId,
  //     );
  //   }

  //   console.log(`...current own orders on OB`);
  //   orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
  //     client,
  //     group,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //   );
  //   for (const order of orders) {
  //     console.log(order);
  //   }

  //   console.log(`...settling funds`);
  //   await client.serum3SettleFunds(
  //     group,
  //     mangoAccount,
  //     DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  //   );
  // }

  // eslint-disable-next-line no-constant-condition
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
        toUiDecimalsForQuote(mangoAccount.getAssetsValue(group)!.toNumber()),
    );
    console.log(
      '...mangoAccount.getLiabsVal() ' +
        toUiDecimalsForQuote(mangoAccount.getLiabsValue(group)!.toNumber()),
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

  // eslint-disable-next-line no-constant-condition
  if (true) {
    // eslint-disable-next-line no-inner-declarations
    function getMaxSourceForTokenSwapWrapper(src, tgt): void {
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
  // eslint-disable-next-line no-constant-condition
  if (true) {
    let sig;
    const perpMarket = group.getPerpMarketByName('BTC-PERP');
    const orders = await mangoAccount.loadPerpOpenOrdersForMarket(
      client,
      group,
      perpMarket.perpMarketIndex,
    );
    for (const order of orders) {
      console.log(
        `Current order - ${order.uiPrice} ${order.uiSize} ${order.side}`,
      );
    }
    console.log(`...cancelling all perp orders`);
    sig = await client.perpCancelAllOrders(
      group,
      mangoAccount,
      perpMarket.perpMarketIndex,
      10,
    );
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // oracle pegged
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price = group.banksMapByName.get('BTC')![0].uiPrice!;
      console.log(
        `...placing perp pegged bid ${clientId} at oracle price ${perpMarket.uiPrice}`,
      );
      const sig = await client.perpPlaceOrderPegged(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.bid,
        -5,
        perpMarket.uiPrice + 5,
        0.01,
        price * 0.011,
        clientId,
        PerpOrderType.limit,
        false,
        0,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price = group.banksMapByName.get('BTC')![0].uiPrice!;
      console.log(
        `...placing perp pegged bid ${clientId} at oracle price ${perpMarket.uiPrice}`,
      );
      const sig = await client.perpPlaceOrderPegged(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.ask,
        5,
        perpMarket.uiPrice - 5,
        0.01,
        price * 0.011,
        clientId,
        PerpOrderType.limit,
        false,
        0,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }

    await logBidsAndAsks(client, group);

    sig = await client.perpCancelAllOrders(
      group,
      mangoAccount,
      perpMarket.perpMarketIndex,
      10,
    );
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // scenario 1
    // bid max perp
    try {
      const clientId = Math.floor(Math.random() * 99999);
      await mangoAccount.reload(client);
      await group.reloadAll(client);
      const price =
        group.banksMapByName.get('BTC')![0].uiPrice! -
        Math.floor(Math.random() * 100);
      const quoteQty = mangoAccount.getMaxQuoteForPerpBidUi(
        group,
        perpMarket.perpMarketIndex,
      );
      const baseQty = quoteQty / price;
      console.log(
        ` simHealthRatioWithPerpBidUiChanges - ${mangoAccount.simHealthRatioWithPerpBidUiChanges(
          group,
          perpMarket.perpMarketIndex,
          baseQty,
        )}`,
      );
      console.log(
        `...placing max qty perp bid  clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.bid,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        false,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }
    console.log(`...cancelling all perp orders`);
    sig = await client.perpCancelAllOrders(
      group,
      mangoAccount,
      perpMarket.perpMarketIndex,
      10,
    );
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // bid max perp + some
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price =
        group.banksMapByName.get('BTC')![0].uiPrice! -
        Math.floor(Math.random() * 100);
      const quoteQty =
        mangoAccount.getMaxQuoteForPerpBidUi(
          group,
          perpMarket.perpMarketIndex,
        ) * 1.02;

      const baseQty = quoteQty / price;
      console.log(
        `...placing max qty * 1.02 perp bid clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.bid,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        false,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
      console.log('Errored out as expected');
    }

    // bid max ask
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price =
        group.banksMapByName.get('BTC')![0].uiPrice! +
        Math.floor(Math.random() * 100);
      const baseQty = mangoAccount.getMaxBaseForPerpAskUi(
        group,
        perpMarket.perpMarketIndex,
      );
      console.log(
        ` simHealthRatioWithPerpAskUiChanges - ${mangoAccount.simHealthRatioWithPerpAskUiChanges(
          group,
          perpMarket.perpMarketIndex,
          baseQty,
        )}`,
      );
      const quoteQty = baseQty * price;
      console.log(
        `...placing max qty perp ask clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.ask,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        false,
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
        mangoAccount.getMaxBaseForPerpAskUi(group, perpMarket.perpMarketIndex) *
        1.02;
      const quoteQty = baseQty * price;
      console.log(
        `...placing max qty perp ask * 1.02 clientId ${clientId} at price ${price}, base ${baseQty}, quote ${quoteQty}`,
      );
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.ask,
        price,
        baseQty,
        quoteQty,
        clientId,
        PerpOrderType.limit,
        false,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
      console.log('Errored out as expected');
    }

    console.log(`...cancelling all perp orders`);
    sig = await client.perpCancelAllOrders(
      group,
      mangoAccount,
      perpMarket.perpMarketIndex,
      10,
    );
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // scenario 2
    // make + take orders
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price = group.banksMapByName.get('BTC')![0].uiPrice!;
      console.log(`...placing perp bid ${clientId} at ${price}`);
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.bid,
        price,
        0.01,
        price * 0.01,
        clientId,
        PerpOrderType.limit,
        false,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }
    try {
      const clientId = Math.floor(Math.random() * 99999);
      const price = group.banksMapByName.get('BTC')![0].uiPrice!;
      console.log(`...placing perp ask ${clientId} at ${price}`);
      const sig = await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpMarket.perpMarketIndex,
        PerpOrderSide.ask,
        price,
        0.01,
        price * 0.011,
        clientId,
        PerpOrderType.limit,
        false,
        0, //Date.now() + 200,
        1,
      );
      console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }
    // // should be able to cancel them : know bug
    // console.log(`...cancelling all perp orders`);
    // sig = await client.perpCancelAllOrders(group, mangoAccount, perpMarket.perpMarketIndex, 10);
    // console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);

    // eslint-disable-next-line @typescript-eslint/no-non-null-asserted-optional-chain
    await perpMarket?.loadEventQueue(client)!;
    const fr = perpMarket?.getInstantaneousFundingRateUi(
      await perpMarket.loadBids(client),
      await perpMarket.loadAsks(client),
    );
    console.log(`current funding rate per hour is ${fr}`);

    // eslint-disable-next-line @typescript-eslint/no-non-null-asserted-optional-chain
    const eq = await perpMarket?.loadEventQueue(client)!;
    console.log(
      `raw events - ${JSON.stringify(eq.eventsSince(new BN(0)), null, 2)}`,
    );

    // sleep so that keeper can catch up
    await new Promise((r) => setTimeout(r, 2000));

    // make+take orders should have cancelled each other, and if keeper has already cranked, then should not appear in position or we see a small quotePositionNative
    await group.reloadAll(client);
    await mangoAccount.reload(client);
    console.log(`${mangoAccount.toString(group)}`);
  }

  process.exit();
}

// eslint-disable-next-line @typescript-eslint/explicit-function-return-type
async function logBidsAndAsks(client: MangoClient, group: Group) {
  await group.reloadAll(client);
  const perpMarket = group.getPerpMarketByName('BTC-PERP');
  const res = [
    (await perpMarket?.loadBids(client)).items(),
    // eslint-disable-next-line @typescript-eslint/no-non-null-asserted-optional-chain
    (await perpMarket?.loadAsks(client)!).items(),
  ];
  console.log(`bids ${JSON.stringify(Array.from(res[0]), null, 2)}`);
  console.log(`asks ${JSON.stringify(Array.from(res[1]), null, 2)}`);
  return res;
}

main();
