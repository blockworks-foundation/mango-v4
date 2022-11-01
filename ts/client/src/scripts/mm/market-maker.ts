import { AnchorProvider, BN, Wallet } from '@project-serum/anchor';
import {
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import fs from 'fs';
import path from 'path';
import { Group } from '../../accounts/group';
import { MangoAccount } from '../../accounts/mangoAccount';
import {
  BookSide,
  PerpMarket,
  PerpMarketIndex,
  PerpOrderSide,
  PerpOrderType,
} from '../../accounts/perp';
import { MangoClient } from '../../client';
import { MANGO_V4_ID } from '../../constants';
import { toUiDecimalsForQuote } from '../../utils';
import { sendTransaction } from '../../utils/rpc';
import {
  makeCheckAndSetSequenceNumberIx,
  makeInitSequenceEnforcerAccountIx,
  seqEnforcerProgramIds,
} from './sequence-enforcer-util';

// TODO switch to more efficient async logging if available in nodejs

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
  market: PerpMarket;
  bids: BookSide;
  asks: BookSide;
  lastBookUpdate: number;

  aggBid: number | undefined;
  aggAsk: number | undefined;
  ftxMid: number | undefined;

  sequenceAccount: PublicKey;
  sequenceAccountBump: number;

  sentBidPrice: number;
  sentAskPrice: number;
  lastOrderUpdate: number;
};

// Refresh mango account and perp market order books
async function refreshState(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  marketContexts: Map<PerpMarketIndex, MarketContext>,
): Promise<State> {
  const ts = Date.now() / 1000;

  // TODO do all updates in one RPC call
  await Promise.all([group.reloadAll(client), mangoAccount.reload(client)]);

  for (const perpMarket of Array.from(
    group.perpMarketsMapByMarketIndex.values(),
  )) {
    const mc = marketContexts.get(perpMarket.perpMarketIndex)!;
    mc.market = perpMarket;
    mc.bids = await perpMarket.loadBids(client);
    mc.asks = await perpMarket.loadAsks(client);
    mc.lastBookUpdate = ts;
  }

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
      mc.market.name,
      CLUSTER,
    ),
  );
  while (true) {
    try {
      const sig = await sendTransaction(
        client.program.provider as AnchorProvider,
        seqAccIxs,
        [],
      );
      console.log(
        `Sequence enforcer accounts created, sig https://explorer.solana.com/tx/${sig}?cluster=${
          CLUSTER == 'devnet' ? 'devnet' : ''
        }`,
      );
    } catch (e) {
      console.log('Failed to initialize sequence enforcer accounts!');
      console.log(e);
      continue;
    }
    break;
  }
}

// Cancel all orders on exit
async function onExit(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  marketContexts: MarketContext[],
) {
  const ixs: TransactionInstruction[] = [];
  for (const mc of marketContexts) {
    const cancelAllIx = await client.perpCancelAllOrdersIx(
      group,
      mangoAccount,
      mc.market.perpMarketIndex,
      10,
    );
    ixs.push(cancelAllIx);
  }
  await sendTransaction(client.program.provider as AnchorProvider, ixs, []);
}

// Main driver for the market maker
async function fullMarketMaker() {
  // Load client
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(USER_KEYPAIR!, 'utf-8'))),
  );
  // TODO: make work for delegate
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {},
    'get-program-accounts',
  );

  // Load mango account
  let mangoAccount = await client.getMangoAccountForPublicKey(
    new PublicKey(MANGO_ACCOUNT_PK),
  );
  console.log(
    `MangoAccount ${mangoAccount.publicKey} for user ${user.publicKey}`,
  );
  await mangoAccount.reload(client);

  // Load group
  const group = await client.getGroup(mangoAccount.group);
  await group.reloadAll(client);

  // Cancel all existing orders
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
  for (const perpMarket of Array.from(
    group.perpMarketsMapByMarketIndex.values(),
  )) {
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
      params: params.assets[perpMarket.name.replace('-PERP', '')].perp,
      market: perpMarket,
      bids: await perpMarket.loadBids(client),
      asks: await perpMarket.loadAsks(client),
      lastBookUpdate: 0,

      sequenceAccount,
      sequenceAccountBump,

      sentBidPrice: 0,
      sentAskPrice: 0,
      lastOrderUpdate: 0,

      // TODO
      aggBid: undefined,
      aggAsk: undefined,
      ftxMid: undefined,
    });
  }

  // Init sequence enforcer accounts
  await initSequenceEnforcerAccounts(
    client,
    Array.from(marketContexts.values()),
  );

  // Load state first time
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
      // TODO update this in a non blocking manner
      state = await refreshState(client, group, mangoAccount, marketContexts);

      mangoAccount = state.mangoAccount;

      // Calculate pf level values
      let pfQuoteValue: number | undefined = 0;
      for (const mc of Array.from(marketContexts.values())) {
        const pos = mangoAccount.getPerpPositionUi(
          group,
          mc.market.perpMarketIndex,
        );
        // TODO use ftx to get mid then also combine with books from other exchanges
        const midWorkaround = mc.market.uiPrice;
        if (midWorkaround) {
          pfQuoteValue += pos * midWorkaround;
        } else {
          pfQuoteValue = undefined;
          console.log(
            `Breaking pfQuoteValue computation, since mid is undefined for ${mc.market.name}!`,
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

        // TODO: batch ixs
        const sig = await sendTransaction(
          client.program.provider as AnchorProvider,
          ixs,
          group.addressLookupTablesList,
        );
        console.log(
          `Orders for market updated, sig https://explorer.solana.com/tx/${sig}?cluster=${
            CLUSTER == 'devnet' ? 'devnet' : ''
          }`,
        );
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
  const perpMarketIndex = mc.market.perpMarketIndex;
  const perpMarket = mc.market;

  const aggBid = perpMarket.uiPrice; // TODO mc.aggBid;
  const aggAsk = perpMarket.uiPrice; // TODO mc.aggAsk;
  if (aggBid === undefined || aggAsk === undefined) {
    console.log(`No Aggregate Book for ${mc.market.name}!`);
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
  // TODO look at event queue as well for unprocessed fills
  const basePos = mangoAccount.getPerpPositionUi(group, perpMarketIndex);
  const lean = (-leanCoeff * basePos) / size;
  const pfQuoteLeanCoeff = params.pfQuoteLeanCoeff || 0.001; // How much to move if pf pos is equal to equity
  const pfQuoteLean = (pfQuoteValue / equity) * -pfQuoteLeanCoeff;
  const charge = (mc.params.charge || 0.0015) + aggSpread / 2;
  const bias = mc.params.bias;
  const bidPrice = fairValue * (1 - charge + lean + bias + pfQuoteLean);
  const askPrice = fairValue * (1 + charge + lean + bias + pfQuoteLean);

  // TODO volatility adjustment

  const modelBidPrice = perpMarket.uiPriceToLots(bidPrice);
  const nativeBidSize = perpMarket.uiBaseToLots(size);
  const modelAskPrice = perpMarket.uiPriceToLots(askPrice);
  const nativeAskSize = perpMarket.uiBaseToLots(size);

  const bids = mc.bids;
  const asks = mc.asks;
  const bestBid = bids.best();
  const bestAsk = asks.best();
  const bookAdjBid =
    bestAsk !== undefined
      ? BN.min(bestAsk.priceLots.sub(new BN(1)), modelBidPrice)
      : modelBidPrice;
  const bookAdjAsk =
    bestBid !== undefined
      ? BN.max(bestBid.priceLots.add(new BN(1)), modelAskPrice)
      : modelAskPrice;

  // TODO use order book to requote if size has changed

  // TODO
  // const takeSpammers = mc.params.takeSpammers;
  // const spammerCharge = mc.params.spammerCharge;

  let moveOrders = false;
  if (mc.lastBookUpdate >= mc.lastOrderUpdate + 2) {
    // console.log(` - moveOrders - 303`);
    // If mango book was updated recently, then MangoAccount was also updated
    const openOrders = await mangoAccount.loadPerpOpenOrdersForMarket(
      client,
      group,
      perpMarketIndex,
    );
    moveOrders = openOrders.length < 2 || openOrders.length > 2;
    for (const o of openOrders) {
      const refPrice = o.side === 'buy' ? bookAdjBid : bookAdjAsk;
      moveOrders =
        moveOrders ||
        Math.abs(o.priceLots.toNumber() / refPrice.toNumber() - 1) >
          requoteThresh;
    }
  } else {
    // console.log(
    //   ` - moveOrders - 319, mc.lastBookUpdate ${mc.lastBookUpdate}, mc.lastOrderUpdate ${mc.lastOrderUpdate}`,
    // );
    // If order was updated before MangoAccount, then assume that sent order already executed
    moveOrders =
      moveOrders ||
      Math.abs(mc.sentBidPrice / bookAdjBid.toNumber() - 1) > requoteThresh ||
      Math.abs(mc.sentAskPrice / bookAdjAsk.toNumber() - 1) > requoteThresh;
  }

  // Start building the transaction
  const instructions: TransactionInstruction[] = [
    makeCheckAndSetSequenceNumberIx(
      mc.sequenceAccount,
      (client.program.provider as AnchorProvider).wallet.publicKey,
      Date.now(),
      CLUSTER,
    ),
  ];

  // TODO Clear 1 lot size orders at the top of book that bad people use to manipulate the price

  if (moveOrders) {
    // Cancel all, requote
    const cancelAllIx = await client.perpCancelAllOrdersIx(
      group,
      mangoAccount,
      perpMarketIndex,
      10,
    );

    const expiryTimestamp =
      params.tif !== undefined ? Date.now() / 1000 + params.tif : 0;

    const placeBidIx = await client.perpPlaceOrderIx(
      group,
      mangoAccount,
      perpMarketIndex,
      PerpOrderSide.bid,
      // TODO fix this, native to ui to native
      perpMarket.priceLotsToUi(bookAdjBid),
      perpMarket.baseLotsToUi(nativeBidSize),
      undefined,
      Date.now(),
      PerpOrderType.postOnlySlide,
      expiryTimestamp,
      20,
    );

    const placeAskIx = await client.perpPlaceOrderIx(
      group,
      mangoAccount,
      perpMarketIndex,
      PerpOrderSide.ask,
      perpMarket.priceLotsToUi(bookAdjAsk),
      perpMarket.baseLotsToUi(nativeAskSize),
      undefined,
      Date.now(),
      PerpOrderType.postOnlySlide,
      expiryTimestamp,
      20,
    );

    instructions.push(cancelAllIx);
    const posAsTradeSizes = basePos / size;
    if (posAsTradeSizes < 15) {
      instructions.push(placeBidIx);
    }
    if (posAsTradeSizes > -15) {
      instructions.push(placeAskIx);
    }
    console.log(
      `Requoting for market ${mc.market.name} sentBid: ${
        mc.sentBidPrice
      } newBid: ${bookAdjBid} sentAsk: ${
        mc.sentAskPrice
      } newAsk: ${bookAdjAsk} pfLean: ${(pfQuoteLean * 10000).toFixed(
        1,
      )} aggBid: ${aggBid} addAsk: ${aggAsk}`,
    );
    mc.sentBidPrice = bookAdjBid.toNumber();
    mc.sentAskPrice = bookAdjAsk.toNumber();
    mc.lastOrderUpdate = Date.now() / 1000;
  } else {
    console.log(
      `Not requoting for market ${mc.market.name}. No need to move orders`,
    );
  }

  // If instruction is only the sequence enforcement, then just send empty
  if (instructions.length === 1) {
    return [];
  } else {
    return instructions;
  }
}

function startMarketMaker() {
  if (control.isRunning) {
    fullMarketMaker().finally(startMarketMaker);
  }
}

startMarketMaker();
