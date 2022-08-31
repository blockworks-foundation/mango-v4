import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { I80F48 } from '../accounts/I80F48';
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
  console.log(`${group}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString(group));

  await mangoAccount.reload(client, group);

  if (false) {
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

  if (false) {
    // expand account
    console.log(
      `...expanding mango account to have serum3 and perp position slots`,
    );
    await client.expandMangoAccount(group, mangoAccount, 16, 8, 8, 8);
    await mangoAccount.reload(client, group);
  }

  if (false) {
    // deposit and withdraw

    try {
      console.log(`...depositing 50 USDC, 1 SOL, 1 MNGO`);
      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('USDC')!),
        50,
      );
      await mangoAccount.reload(client, group);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('SOL')!),
        1,
      );
      await mangoAccount.reload(client, group);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('MNGO')!),
        1,
      );
      await mangoAccount.reload(client, group);

      console.log(`...withdrawing 1 USDC`);
      await client.tokenWithdraw(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('USDC')!),
        1,
        true,
      );
      await mangoAccount.reload(client, group);

      console.log(`...depositing 0.0005 BTC`);
      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('BTC')!),
        0.0005,
      );
      await mangoAccount.reload(client, group);

      console.log(mangoAccount.toString(group));
    } catch (error) {
      console.log(error);
    }
  }

  if (false) {
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
    await mangoAccount.reload(client, group);

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
    await mangoAccount.reload(client, group);

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

  if (false) {
    // serum3 market
    const serum3Market = group.serum3MarketsMapByExternal.get(
      DEVNET_SERUM3_MARKETS.get('BTC/USDC')!.toBase58(),
    );
    console.log(await serum3Market?.logOb(client, group));
  }

  if (false) {
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
          mangoAccount.getAssetsValue(HealthType.init).toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getLiabsVal() ' +
        toUiDecimalsForQuote(
          mangoAccount.getLiabsValue(HealthType.init).toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getMaxWithdrawWithBorrowForToken(group, "SOL") ' +
        toUiDecimalsForQuote(
          (
            await mangoAccount.getMaxWithdrawWithBorrowForToken(
              group,
              new PublicKey(DEVNET_MINTS.get('SOL')!),
            )
          ).toNumber(),
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
          mangoAccount
            .getMaxSourceForTokenSwap(
              group,
              group.banksMapByName.get(src)![0].mint,
              group.banksMapByName.get(tgt)![0].mint,
              1,
            )
            .div(
              I80F48.fromNumber(
                Math.pow(10, group.banksMapByName.get(src)![0].mintDecimals),
              ),
            )
            .toNumber(),
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

  if (false) {
    console.log(
      "...mangoAccount.getPerpMarketMarginAvailable(group, 'BTC-PERP') " +
        toUiDecimalsForQuote(
          mangoAccount
            .getPerpMarketMarginAvailable(group, 'BTC-PERP')
            .toNumber(),
        ),
    );
  }

  if (false) {
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
