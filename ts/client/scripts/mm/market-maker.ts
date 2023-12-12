import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import {
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import Binance from 'binance-api-node';
import fs from 'fs';
import { Kraken } from 'node-kraken-api';
import path from 'path';
import { Group } from '../../src/accounts/group';
import { HealthType, MangoAccount } from '../../src/accounts/mangoAccount';
import {
  BookSide,
  PerpMarket,
  PerpMarketIndex,
  PerpOrderSide,
  PerpOrderType,
  PerpSelfTradeBehavior,
} from '../../src/accounts/perp';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { toUiDecimalsForQuote } from '../../src/utils';
import { sendTransaction } from '../../src/utils/rpc';
import * as defaultParams from './params/default.json';
import {
  makeCheckAndSetSequenceNumberIx,
  makeInitSequenceEnforcerAccountIx,
  seqEnforcerProgramIds,
} from './sequence-enforcer-util';

console.log(defaultParams);

// Future
// * use async nodejs logging
// * merge gMa calls
// * take out spammers
// * batch ixs across various markets
// * only refresh part of the group which market maker is interested in

// Env vars
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';

// Load configuration
const paramsFileName = process.env.PARAMS || 'default.json';
const params = JSON.parse(
  fs.readFileSync(
    path.resolve(__dirname, `./params/${paramsFileName}`),
    'utf-8',
  ),
);

const control = { isRunning: true, interval: params.interval };

// State which is passed around
type State = {
  mangoAccount: MangoAccount;
  lastMangoAccountUpdate: number;
  marketContexts: Map<PerpMarketIndex, MarketContext>;
};
type MarketContext = {
  params: any;
  perpMarket: PerpMarket;
  bids: BookSide;
  asks: BookSide;
  lastBookUpdate: number;

  krakenBid: number | undefined;
  krakenAsk: number | undefined;

  // binanceBid: number | undefined;
  // binanceAsk: number | undefined;

  sequenceAccount: PublicKey;
  sequenceAccountBump: number;

  sentBidPrice: number;
  sentAskPrice: number;
  lastOrderUpdate: number;
};

const binanceClient = Binance();
const krakenClient = new Kraken();

function getPerpMarketAssetsToTradeOn(group: Group) {
  const allMangoGroupPerpMarketAssets = Array.from(
    group.perpMarketsMapByName.keys(),
  ).map((marketName) => marketName.replace('-PERP', ''));
  return Object.keys(params.assets).filter((asset) =>
    allMangoGroupPerpMarketAssets.includes(asset),
  );
}

// Refresh group, mango account and perp markets
async function refreshState(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  marketContexts: Map<PerpMarketIndex, MarketContext>,
): Promise<State> {
  const ts = Date.now() / 1000;

  const result = await Promise.all([
    group.reloadAll(client),
    mangoAccount.reload(client),
    ...Array.from(marketContexts.values()).map(
      (mc) =>
        krakenClient.depth({
          pair: mc.params.krakenCode,
        }),
      // binanceClient.book({
      //   symbol: mc.perpMarket.name.replace('-PERP', 'USDT'),
      // }),
    ),
  ]);

  Array.from(marketContexts.values()).map(async (mc, i) => {
    const perpMarket = mc.perpMarket;
    mc.perpMarket = group.getPerpMarketByMarketIndex(
      perpMarket.perpMarketIndex,
    );
    mc.bids = await perpMarket.loadBids(client, true);
    mc.asks = await perpMarket.loadAsks(client, true);
    mc.lastBookUpdate = ts;

    mc.krakenAsk = parseFloat(
      (result[i + 2] as any)[mc.params.krakenCode].asks[0][0],
    );
    mc.krakenBid = parseFloat(
      (result[i + 2] as any)[mc.params.krakenCode].bids[0][0],
    );
  });

  return {
    mangoAccount,
    lastMangoAccountUpdate: ts,
    marketContexts,
  };
}

// Initialiaze sequence enforcer accounts
async function initSequenceEnforcerAccounts(
  client: MangoClient,
  marketContexts: MarketContext[],
) {
  const seqAccIxs = marketContexts.map((mc) =>
    makeInitSequenceEnforcerAccountIx(
      mc.sequenceAccount,
      (client.program.provider as AnchorProvider).wallet.publicKey,
      mc.sequenceAccountBump,
      mc.perpMarket.name,
      CLUSTER,
    ),
  );

  // eslint-disable-next-line
  while (true) {
    try {
      const sig = await sendTransaction(
        client.program.provider as AnchorProvider,
        seqAccIxs,
        [],
      );
      console.log(
        `Sequence enforcer accounts created, sig https://explorer.solana.com/tx/${
          sig.signature
        }?cluster=${CLUSTER == 'devnet' ? 'devnet' : ''}`,
      );
    } catch (e) {
      console.log('Failed to initialize sequence enforcer accounts!');
      console.log(e);
      continue;
    }
    break;
  }
}

async function cancelAllOrdersForAMarket(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  perpMarket: PerpMarket,
) {
  for (const i of Array(100).keys()) {
    await sendTransaction(
      client.program.provider as AnchorProvider,
      [
        await client.perpCancelAllOrdersIx(
          group,
          mangoAccount,
          perpMarket.perpMarketIndex,
          10,
        ),
      ],
      [],
    );
    await mangoAccount.reload(client);
    if (
      (
        await mangoAccount.loadPerpOpenOrdersForMarket(
          client,
          group,
          perpMarket.perpMarketIndex,
        )
      ).length === 0
    ) {
      break;
    }
  }
}

// Cancel all orders on exit
async function onExit(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  marketContexts: MarketContext[],
) {
  for (const mc of marketContexts) {
    cancelAllOrdersForAMarket(client, group, mangoAccount, mc.perpMarket);
  }
}

// Main driver for the market maker
async function fullMarketMaker() {
  // Load client
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  // Load mango account
  let mangoAccount = await client.getMangoAccount(
    new PublicKey(MANGO_ACCOUNT_PK),
  );
  console.log(
    `MangoAccount ${mangoAccount.publicKey} for user ${user.publicKey} ${
      mangoAccount.isDelegate(client) ? 'via delegate ' + user.publicKey : ''
    }`,
  );
  await mangoAccount.reload(client);

  // Load group
  const group = await client.getGroup(mangoAccount.group);
  await group.reloadAll(client);

  // Cancel all existing orders
  console.log(`Cancelling all orders...`);
  for (const perpMarket of Array.from(
    group.perpMarketsMapByMarketIndex.values(),
  )) {
    await client.perpCancelAllOrders(
      group,
      mangoAccount,
      perpMarket.perpMarketIndex,
      10,
    );
  }

  // Build and maintain an aggregate context object per market
  const marketContexts: Map<PerpMarketIndex, MarketContext> = new Map();
  for (const perpMarketAsset of getPerpMarketAssetsToTradeOn(group)) {
    const perpMarket = group.getPerpMarketByName(perpMarketAsset + '-PERP');
    const [sequenceAccount, sequenceAccountBump] =
      await PublicKey.findProgramAddress(
        [
          Buffer.from(perpMarket.name, 'utf-8'),
          (
            client.program.provider as AnchorProvider
          ).wallet.publicKey.toBytes(),
        ],
        seqEnforcerProgramIds[CLUSTER],
      );
    marketContexts.set(perpMarket.perpMarketIndex, {
      params: params.assets[perpMarketAsset].perp,
      perpMarket: perpMarket,
      bids: await perpMarket.loadBids(client),
      asks: await perpMarket.loadAsks(client),
      lastBookUpdate: 0,

      sequenceAccount,
      sequenceAccountBump,

      sentBidPrice: 0,
      sentAskPrice: 0,
      lastOrderUpdate: 0,

      krakenBid: undefined,
      krakenAsk: undefined,
    });
  }

  // Init sequence enforcer accounts
  console.log(`Init sequence enforcer...`);
  await initSequenceEnforcerAccounts(
    client,
    Array.from(marketContexts.values()),
  );

  // Load state first time
  console.log(`Loading state first time`);
  let state = await refreshState(client, group, mangoAccount, marketContexts);

  // Add handler for e.g. CTRL+C
  process.on('SIGINT', function () {
    console.log('Caught keyboard interrupt. Canceling orders');
    control.isRunning = false;
    onExit(client, group, mangoAccount, Array.from(marketContexts.values()));
  });

  // Loop indefinitely
  while (control.isRunning) {
    try {
      console.log(`\nRefreshing state`);
      refreshState(client, group, mangoAccount, marketContexts).then(
        (result) => (state = result),
      );

      mangoAccount = state.mangoAccount;

      // Calculate pf level values
      let pfQuoteValue: number | undefined = 0;
      for (const mc of Array.from(marketContexts.values())) {
        const pos = mangoAccount.perpPositionExistsForMarket(mc.perpMarket)
          ? mangoAccount.getPerpPositionUi(group, mc.perpMarket.perpMarketIndex)
          : 0;
        const mid = (mc.krakenBid! + mc.krakenAsk!) / 2;
        if (mid) {
          pfQuoteValue += pos * mid;
        } else {
          pfQuoteValue = undefined;
          console.log(
            `Breaking pfQuoteValue computation, since mid is undefined for ${mc.perpMarket.name}!`,
          );
          break;
        }
      }

      // Don't proceed if we don't have pfQuoteValue yet
      if (pfQuoteValue === undefined) {
        console.log(
          `Continuing control loop, since pfQuoteValue is undefined!`,
        );
        continue;
      }

      // Update all orders on all markets
      for (const mc of Array.from(marketContexts.values())) {
        const basePos = mangoAccount.perpPositionExistsForMarket(mc.perpMarket)
          ? mangoAccount.getPerpPositionUi(
              group,
              mc.perpMarket.perpMarketIndex,
              true,
            )
          : 0;
        if (basePos !== 0) {
          const equityBy100 =
            toUiDecimalsForQuote(mangoAccount.getEquity(group)) /
            (100 * mc.perpMarket.uiPrice);
          const posSizeToClose = Math.abs(Math.max(basePos / 10, equityBy100));
          const sig = await client.perpPlaceOrder(
            group,
            mangoAccount,
            mc.perpMarket.perpMarketIndex,
            basePos > 0 ? PerpOrderSide.ask : PerpOrderSide.bid,
            mc.perpMarket.uiPrice * (basePos > 0 ? 1 - 0.01 : 1 + 0.01),
            posSizeToClose,
            undefined,
            Date.now(),
            PerpOrderType.immediateOrCancel,
            true,
          );
          console.log(
            `Twap closing position (current ${basePos}, equityBy100 ${equityBy100}, closing ${posSizeToClose}), sig https://explorer.solana.com/tx/${
              sig.signature
            }?cluster=${CLUSTER == 'devnet' ? 'devnet' : ''}`,
          );
        } else {
          const ixs = await makeMarketUpdateInstructions(
            client,
            group,
            mangoAccount,
            mc,
            pfQuoteValue,
          );
          if (ixs.length === 0) {
            continue;
          }

          const sig = await sendTransaction(
            client.program.provider as AnchorProvider,
            ixs,
            group.addressLookupTablesList,
          );
          console.log(
            `Orders for market updated, sig https://explorer.solana.com/tx/${
              sig.signature
            }?cluster=${CLUSTER == 'devnet' ? 'devnet' : ''}`,
          );
        }
      }
    } catch (e) {
      console.log(e);
    } finally {
      console.log(
        `${new Date().toUTCString()} sleeping for ${control.interval / 1000}s`,
      );
      await new Promise((r) => setTimeout(r, control.interval));
    }
  }
}

async function makeMarketUpdateInstructions(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  mc: MarketContext,
  pfQuoteValue: number,
): Promise<TransactionInstruction[]> {
  const perpMarketIndex = mc.perpMarket.perpMarketIndex;
  const perpMarket = mc.perpMarket;

  const aggBid = mc.krakenBid;
  const aggAsk = mc.krakenAsk;
  if (aggBid === undefined || aggAsk === undefined) {
    console.log(`No Aggregate Book for ${mc.perpMarket.name}!`);
    return [];
  }

  const leanCoeff = mc.params.leanCoeff;

  const fairValue = (aggBid + aggAsk) / 2;
  const aggSpread = (aggAsk - aggBid) / fairValue;

  const requoteThresh = mc.params.requoteThresh;
  const equity = toUiDecimalsForQuote(mangoAccount.getEquity(group));
  const sizePerc = mc.params.sizePerc;
  const quoteSize = equity * sizePerc;
  const size = quoteSize / fairValue;

  // console.log(`equity ${equity}`);
  // console.log(`sizePerc ${sizePerc}`);
  // console.log(`fairValue ${fairValue}`);
  // console.log(`size ${size}`);

  const basePos = mangoAccount.perpPositionExistsForMarket(mc.perpMarket)
    ? mangoAccount.getPerpPositionUi(group, perpMarketIndex, true)
    : 0;
  const unsettledPnl = mangoAccount.perpPositionExistsForMarket(mc.perpMarket)
    ? mangoAccount
        .getPerpPosition(perpMarketIndex)!
        .getUnsettledPnlUi(perpMarket)
    : 0;
  const lean = (-leanCoeff * basePos) / size;
  const pfQuoteLeanCoeff = params.pfQuoteLeanCoeff || 0.001; // How much to move if pf pos is equal to equity
  const pfQuoteLean = (pfQuoteValue / equity) * -pfQuoteLeanCoeff;
  const charge = (mc.params.charge || 0.0012) + aggSpread / 2;
  const bias = mc.params.bias;

  const fairValueInLots = perpMarket.uiPriceToLots(fairValue);

  const nativeBidSize = perpMarket.uiBaseToLots(size);
  const nativeAskSize = perpMarket.uiBaseToLots(size);

  const bids = mc.bids;
  const asks = mc.asks;
  const bestBid = bids.best();
  const bestAsk = asks.best();

  let moveOrders = false;

  // Start building the transaction
  const instructions: TransactionInstruction[] = [
    makeCheckAndSetSequenceNumberIx(
      mc.sequenceAccount,
      (client.program.provider as AnchorProvider).wallet.publicKey,
      Date.now(),
      CLUSTER,
    ),
  ];

  // Temporary: Health regions are currently disabled on mainnet for security reasons
  //instructions.push(
  //  await client.healthRegionBeginIx(group, mangoAccount, [], [perpMarket]),
  //);

  // Trigger update funding before cancelling existing orders
  instructions.push(await client.perpUpdateFundingIx(group, perpMarket));

  const expiryTimestamp =
    params.tif !== undefined ? Date.now() / 1000 + params.tif : 0;

  // TODO: oracle pegged runs out of free perp open order slots on mango account
  if (params.oraclePegged) {
    const uiOPegBidOffset =
      fairValue *
      (-charge -
        perpMarket.baseLiquidationFee.toNumber() * 0.9 +
        +lean +
        bias +
        pfQuoteLean);
    const uiOPegAskOffset =
      fairValue *
      (charge +
        perpMarket.baseLiquidationFee.toNumber() * 0.9 +
        lean +
        bias +
        pfQuoteLean);

    const modelBidOPegOffset = perpMarket.uiPriceToLots(uiOPegBidOffset);
    const modelAskOPegOffset = perpMarket.uiPriceToLots(uiOPegAskOffset);

    const bookAdjBidOPegOffset = bestAsk?.priceLots
      .sub(new BN(1))
      .lt(fairValueInLots.add(modelBidOPegOffset))
      ? fairValueInLots.sub(bestAsk.priceLots.sub(new BN(1)))
      : modelBidOPegOffset;
    const bookAdjAskOPegOffset = bestBid?.priceLots
      .add(new BN(1))
      .gt(fairValueInLots.add(modelAskOPegOffset))
      ? bestBid.priceLots.sub(new BN(1)).sub(fairValueInLots)
      : modelAskOPegOffset;

    const openOrders = await mangoAccount.loadPerpOpenOrdersForMarket(
      client,
      group,
      perpMarketIndex,
    );

    moveOrders = openOrders.length < 2;

    const placeBidOPegIx = await client.perpPlaceOrderPeggedV2Ix(
      group,
      mangoAccount,
      perpMarketIndex,
      PerpOrderSide.bid,
      perpMarket.priceLotsToUi(bookAdjBidOPegOffset),
      perpMarket.priceLotsToUi(
        fairValueInLots.mul(new BN(101)).div(new BN(100)),
      ),
      perpMarket.baseLotsToUi(nativeBidSize),
      undefined,
      Date.now(),
      PerpOrderType.limit,
      PerpSelfTradeBehavior.cancelProvide,
      false,
      expiryTimestamp,
      20,
    );

    const placeAskOPegIx = await client.perpPlaceOrderPeggedV2Ix(
      group,
      mangoAccount,
      perpMarketIndex,
      PerpOrderSide.ask,
      perpMarket.priceLotsToUi(bookAdjAskOPegOffset),
      perpMarket.priceLotsToUi(
        fairValueInLots.mul(new BN(98)).div(new BN(100)),
      ),
      perpMarket.baseLotsToUi(nativeAskSize),
      undefined,
      Date.now(),
      PerpOrderType.limit,
      PerpSelfTradeBehavior.cancelProvide,
      false,
      expiryTimestamp,
      20,
    );

    const posAsTradeSizes = basePos / size;

    // console.log(
    //   `basePos ${basePos}, posAsTradeSizes ${posAsTradeSizes}, size ${size}`,
    // );

    if (posAsTradeSizes < 15) {
      instructions.push(placeBidOPegIx);
    }
    if (posAsTradeSizes > -15) {
      instructions.push(placeAskOPegIx);
    }

    const approxOPegBidPrice = perpMarket.priceLotsToUi(
      fairValueInLots.add(bookAdjBidOPegOffset),
    );
    const approxOPegAskPrice = perpMarket.priceLotsToUi(
      fairValueInLots.add(bookAdjAskOPegOffset),
    );

    if (posAsTradeSizes < 15 || posAsTradeSizes > -15) {
      console.log(
        `Requoting for market ${mc.perpMarket.name} sentBid: ${
          mc.sentBidPrice
        } newBid: ${approxOPegBidPrice} sentAsk: ${
          mc.sentAskPrice
        } newAsk: ${approxOPegAskPrice} pfLean: ${(pfQuoteLean * 10000).toFixed(
          1,
        )} aggBid: ${aggBid} addAsk: ${aggAsk}`,
      );
      mc.sentBidPrice = approxOPegAskPrice;
      mc.sentAskPrice = approxOPegAskPrice;
      mc.lastOrderUpdate = Date.now() / 1000;
    }
  } else {
    const uiBidPrice =
      fairValue *
      (1 -
        charge -
        perpMarket.baseLiquidationFee.toNumber() * 0.9 +
        lean +
        bias +
        pfQuoteLean);
    const uiAskPrice =
      fairValue *
      (1 +
        charge +
        perpMarket.baseLiquidationFee.toNumber() * 0.9 +
        lean +
        bias +
        pfQuoteLean);

    const modelBidPrice = perpMarket.uiPriceToLots(uiBidPrice);
    const modelAskPrice = perpMarket.uiPriceToLots(uiAskPrice);

    const bookAdjBid =
      bestAsk !== undefined
        ? BN.min(bestAsk.priceLots.sub(new BN(1)), modelBidPrice)
        : modelBidPrice;
    const bookAdjAsk =
      bestBid !== undefined
        ? BN.max(bestBid.priceLots.add(new BN(1)), modelAskPrice)
        : modelAskPrice;

    if (mc.lastBookUpdate >= mc.lastOrderUpdate + 2) {
      // If mango book was updated recently, then MangoAccount was also updated
      const openOrders = await mangoAccount.loadPerpOpenOrdersForMarket(
        client,
        group,
        perpMarketIndex,
      );
      moveOrders = openOrders.length < 2 || openOrders.length > 2;
      for (const o of openOrders) {
        const refPrice = o.side === PerpOrderSide.bid ? bookAdjBid : bookAdjAsk;
        moveOrders =
          moveOrders ||
          Math.abs(o.priceLots.toNumber() / refPrice.toNumber() - 1) >
            requoteThresh;
      }
    } else {
      // If order was updated before MangoAccount, then assume that sent order already executed
      moveOrders =
        moveOrders ||
        Math.abs(mc.sentBidPrice / bookAdjBid.toNumber() - 1) > requoteThresh ||
        Math.abs(mc.sentAskPrice / bookAdjAsk.toNumber() - 1) > requoteThresh;
    }

    if (moveOrders) {
      // Cancel all, requote
      const cancelAllIx = await client.perpCancelAllOrdersIx(
        group,
        mangoAccount,
        perpMarketIndex,
        10,
      );

      const placeBidIx = await client.perpPlaceOrderV2Ix(
        group,
        mangoAccount,
        perpMarketIndex,
        PerpOrderSide.bid,
        perpMarket.priceLotsToUi(bookAdjBid),
        perpMarket.baseLotsToUi(nativeBidSize),
        undefined,
        Date.now(),
        PerpOrderType.postOnlySlide,
        PerpSelfTradeBehavior.cancelProvide,
        false,
        expiryTimestamp,
        20,
      );

      const placeAskIx = await client.perpPlaceOrderV2Ix(
        group,
        mangoAccount,
        perpMarketIndex,
        PerpOrderSide.ask,
        perpMarket.priceLotsToUi(bookAdjAsk),
        perpMarket.baseLotsToUi(nativeAskSize),
        undefined,
        Date.now(),
        PerpOrderType.postOnlySlide,
        PerpSelfTradeBehavior.cancelProvide,
        false,
        expiryTimestamp,
        20,
      );

      // console.log(
      //   `basePos ${basePos}, posAsTradeSizes ${posAsTradeSizes}, size ${size}`,
      // );

      const posAsTradeSizes = basePos / size;

      instructions.push(cancelAllIx);
      if (posAsTradeSizes < 15) {
        instructions.push(placeBidIx);
      }
      if (posAsTradeSizes > -15) {
        instructions.push(placeAskIx);
      }

      console.log(
        `\nRequoting for market ${mc.perpMarket.name} sentBid: ${
          mc.sentBidPrice
        } newBid: ${bookAdjBid} sentAsk: ${
          mc.sentAskPrice
        } newAsk: ${bookAdjAsk} pfLean: ${(pfQuoteLean * 10000).toFixed(
          1,
        )} aggBid: ${aggBid} addAsk: ${aggAsk}`,
      );

      console.log(
        `Health ratio ${mangoAccount
          .getHealthRatio(group, HealthType.maint)
          .toFixed(3)}, maint health ${toUiDecimalsForQuote(
          mangoAccount.getHealth(group, HealthType.maint),
        ).toFixed(3)}, account equity ${equity.toFixed(
          3,
        )}, base position ${Math.abs(basePos).toFixed(3)} ${
          basePos >= 0 ? 'LONG' : 'SHORT'
        }, notional ${Math.abs(basePos * perpMarket.uiPrice).toFixed(
          3,
        )}, unsettled Pnl ${unsettledPnl.toFixed(3)}`,
      );

      mc.sentBidPrice = bookAdjBid.toNumber();
      mc.sentAskPrice = bookAdjAsk.toNumber();
      mc.lastOrderUpdate = Date.now() / 1000;
    } else {
      console.log(
        `Not requoting for market ${mc.perpMarket.name}. No need to move orders`,
      );
    }
  }

  // Temporary: Health regions are currently disabled on mainnet for security reasons
  //instructions.push(
  //  await client.healthRegionEndIx(group, mangoAccount, [], [perpMarket]),
  //);

  // If instruction is only the sequence enforcement and health region ixs, then just send empty
  if (instructions.length === 3) {
    return [];
  } else {
    return instructions;
  }
}

function startMarketMaker() {
  try {
    if (control.isRunning) {
      fullMarketMaker()
        .catch((error) => console.log(error))
        .finally(startMarketMaker);
    }
  } catch (error) {
    console.log(error);
  }
}

startMarketMaker();
