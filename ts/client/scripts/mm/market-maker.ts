import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import {
  BlockhashWithExpiryBlockHeight,
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import fs from 'fs';
import path from 'path';
import { Bank } from '../../src/accounts/bank';
import { Group } from '../../src/accounts/group';
import { HealthType, MangoAccount } from '../../src/accounts/mangoAccount';
import {
  BookSide,
  FillEvent,
  PerpMarket,
  PerpMarketIndex,
  PerpOrderSide,
  PerpOrderType,
} from '../../src/accounts/perp';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { I80F48 } from '../../src/numbers/I80F48';
import { QUOTE_DECIMALS, toUiDecimalsForQuote } from '../../src/utils';
import { sendTransaction } from '../../src/utils/rpc';
import * as defaultParams from './params/default.json';
import {
  findOwnSeqEnforcerAddress,
  makeCheckAndSetSequenceNumberIx,
  makeInitSequenceEnforcerAccountIx,
} from './sequence-enforcer-util';

console.log(defaultParams);

// Future
// * use async nodejs logging
// * merge gMa calls
// * take out spammers
// * batch ixs across various markets
// * only refresh part of the group which market maker is interested in

// Env vars

const CLUSTER: Cluster = (process.env.CLUSTER as Cluster) || 'mainnet-beta';
const RPC_URL = process.env.RPC_URL;
const KEYPAIR =
  process.env.KEYPAIR || fs.readFileSync(process.env.KEYPAIR_PATH!, 'utf-8');
const MANGO_ACCOUNT = new PublicKey(process.env.MANGO_ACCOUNT!);

// Load configuration
const paramsFileName = process.env.PARAMS || 'default.json';
const params = JSON.parse(
  fs.readFileSync(
    path.resolve(__dirname, `./params/${paramsFileName}`),
    'utf-8',
  ),
);

const control = { isRunning: true };

// Global bot state shared between all markets
class BotContext {
  latestBlockhash: BlockhashWithExpiryBlockHeight;
  latestBlockhashLastUpdatedTs: number;
  groupLastUpdatedTs: number;
  mangoAccountLastUpdatedSlot: number;
  mangoAccountLastUpdatedTs: number;

  constructor(
    public client: MangoClient,
    public group: Group,
    public mangoAccount: MangoAccount,
    public perpMarkets: Map<PerpMarketIndex, MarketContext>,
  ) {}

  async refreshAll(): Promise<any> {
    return Promise.all([
      this.refreshBlockhash(),
      this.refreshGroup(),
      this.refreshMangoAccount(),
    ]);
  }

  async refreshBlockhash(): Promise<void> {
    if (!control.isRunning) return;
    try {
      const response = await this.client.connection.getLatestBlockhash(
        'finalized',
      );
      this.latestBlockhash = response;
      this.latestBlockhashLastUpdatedTs = Date.now();
    } catch (e) {
      console.error('could not refresh blockhash', e);
    }
  }

  async refreshGroup(): Promise<void> {
    if (!control.isRunning) return;
    try {
      await this.group.reloadAll(this.client);
      // sync the context for every perp market
      Array.from(this.perpMarkets.values()).map(async (mc, i) => {
        const perpMarket = mc.perpMarket;
        mc.perpMarket = this.group.getPerpMarketByMarketIndex(
          perpMarket.perpMarketIndex,
        );
      });
      this.groupLastUpdatedTs = Date.now();
    } catch (e) {
      console.error('could not refresh blockhash', e);
    }
  }

  async refreshMangoAccount(): Promise<void> {
    if (!control.isRunning) return;
    try {
      const response = await this.client.getMangoAccountWithSlot(
        this.mangoAccount.publicKey,
        false,
      );
      if (response) {
        this.mangoAccount = response.value;
        this.mangoAccountLastUpdatedSlot = response.slot;
        this.mangoAccountLastUpdatedTs = Date.now();
      } else {
        console.warn('could not fetch mangoAccount');
      }
    } catch (e) {
      console.error('could not refresh mangoAccount', e);
    }
  }

  calculateDelta(i: PerpMarketIndex) {
    const { params, perpMarket } = this.perpMarkets.get(i)!;

    const tokenMint = new PublicKey(params.tokenMint);
    const tokenBank = this.group.getFirstBankByMint(tokenMint);
    const tokenPosition = this.mangoAccount.getTokenBalanceUi(tokenBank);
    const perpPosition = this.mangoAccount.getPerpPositionUi(
      this.group,
      perpMarket.perpMarketIndex,
    );
    // TODO: add fill delta into perpPosition
    return { delta: tokenPosition + perpPosition, tokenPosition, perpPosition };
  }

  async updateOrders(i: PerpMarketIndex): Promise<string> {
    const { params, perpMarket } = this.perpMarkets.get(i)!;
    const { delta } = this.calculateDelta(i);
    const currentEquityInUnderlying =
      toUiDecimalsForQuote(this.mangoAccount.getEquity(this.group).toNumber()) /
      perpMarket.uiPrice;
    const maxSize = currentEquityInUnderlying * params.orderSizeLimit;
    const bidSize = Math.min(maxSize, params.deltaMax - delta);
    const askSize = Math.min(maxSize, delta - params.deltaMin);
    const bidPrice = perpMarket.uiPrice * -params.spreadCharge;
    const askPrice = perpMarket.uiPrice * +params.spreadCharge;

    console.log(
      `update orders delta=${delta} equity=${currentEquityInUnderlying} mark=${perpMarket.uiPrice} bid=${bidSize}@${bidPrice} ask=${askSize}@${askPrice}`,
    );

    const beginIx = await this.client.healthRegionBeginIx(
      this.group,
      this.mangoAccount,
      [],
      [perpMarket],
    );
    const cancelAllIx = await this.client.perpCancelAllOrdersIx(
      this.group,
      this.mangoAccount,
      perpMarket.perpMarketIndex,
      4,
    );
    const bidIx = await this.client.perpPlaceOrderPeggedIx(
      this.group,
      this.mangoAccount,
      perpMarket.perpMarketIndex,
      PerpOrderSide.bid,
      bidPrice,
      bidSize,
      undefined,
      undefined,
      Date.now(),
    );
    const askIx = await this.client.perpPlaceOrderPeggedIx(
      this.group,
      this.mangoAccount,
      perpMarket.perpMarketIndex,
      PerpOrderSide.ask,
      askPrice,
      askSize,
      undefined,
      undefined,
      Date.now(),
    );

    const endIx = await this.client.healthRegionEndIx(
      this.group,
      this.mangoAccount,
      [],
      [perpMarket],
    );

    return this.client.sendAndConfirmTransaction([
      // beginIx,
      cancelAllIx,
      bidIx,
      askIx,
      // endIx,
    ]);
  }

  async hedgeFills(i: PerpMarketIndex): Promise<void> {
    const { params, perpMarket } = this.perpMarkets.get(i)!;

    console.log('start hedger', i);
    while (control.isRunning) {
      const { delta, perpPosition } = this.calculateDelta(i);
      const biasedDelta = delta + params.deltaBias;

      console.log(
        `hedge delta=${delta} perp=${perpPosition} biased=${biasedDelta}`,
      );

      if (Math.abs(biasedDelta) > perpMarket.minOrderSize) {
        // prefer perp hedge if perp position has same sign as account delta
        if (Math.sign(biasedDelta) * Math.sign(perpPosition) > 0) {
          // hedge size is limited to split hedges into reasonable order sizes
          const hedgeSize = Math.min(Math.abs(biasedDelta), params.hedgeMax);
          // hedge price needs to cross the spread and offer a discount
          const hedgePrice =
            perpMarket.uiPrice *
            (1 - Math.sign(biasedDelta) * params.hedgeDiscount);

          const side =
            Math.sign(biasedDelta) > 0 ? PerpOrderSide.ask : PerpOrderSide.bid;

          console.log(
            `hedge perp delta=${biasedDelta} side=${
              side == PerpOrderSide.ask ? 'ask' : 'bid'
            } size=${hedgeSize} limit=${hedgePrice} index=${
              perpMarket.uiPrice
            }`,
          );

          const ix = await this.client.perpPlaceOrderIx(
            this.group,
            this.mangoAccount,
            i,
            side,
            hedgePrice,
            hedgeSize,
            undefined,
            Date.now(),
            PerpOrderType.immediateOrCancel,
            true,
          );

          const confirmBegin = Date.now();
          const sig = await this.client.sendAndConfirmTransaction([ix], {
            alts: this.group.addressLookupTablesList,
            latestBlockhash: this.latestBlockhash,
          });
          const confirmEnd = Date.now();

          const newDelta = this.calculateDelta(i).delta;

          console.log(
            `hedge ${i} confirmed delta time=${
              (confirmEnd - confirmBegin) / 1000
            } prev=${delta} new=${newDelta} https://explorer.solana.com/tx/${sig}`,
          );
        }
      }

      //       hedge to bring delta in line with config goal
      //          spot hedge in parallel with same sequence id but a second tx
      //          if perp hedge fails, sequence id won't be advanced and spot hedge will succeed
      //          spot hedge should use known liquid routes without network requests (mango-router run in parallel)
      //       if perp position would increase:
      //          spot hedge
      //       wait for hedge txs to confirm:
      //          update oracle peg orders

      await sleep(30); // sleep a few ms
    }
  }
}

// market specific state
type MarketContext = {
  params: any;
  perpMarket: PerpMarket;
  sequenceAccount: PublicKey;
  sequenceAccountBump: number;
  collateralBank: Bank;
  unprocessedPerpFills: { fills: FillEvent; slot: number }[];
};

function getPerpMarketAssetsToTradeOn(group: Group): string[] {
  const allMangoGroupPerpMarketAssets = Array.from(
    group.perpMarketsMapByName.keys(),
  ).map((marketName) => marketName.replace('-PERP', ''));
  return Object.keys(params.assets).filter((asset) =>
    allMangoGroupPerpMarketAssets.includes(asset),
  );
}

// Initialiaze sequence enforcer accounts
async function initSequenceEnforcerAccounts(
  client: MangoClient,
  marketContexts: MarketContext[],
): Promise<void> {
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

async function cancelAllOrdersForAMarket(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  perpMarket: PerpMarket,
): Promise<void> {
  console.log('cancel', perpMarket.name);
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
    // TODO: reloading the same account multiple times inside a loop
    //       over all perp markets seems very wasteful
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
  console.log('cancel all');
  for (const mc of marketContexts) {
    // TODO: this doesn't actually enforce execution as promise is never awaited for
    cancelAllOrdersForAMarket(client, group, mangoAccount, mc.perpMarket);
  }
}

function sleep(ms): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// Main driver for the market maker
async function fullMarketMaker(): Promise<void> {
  let intervals: NodeJS.Timer[] = [];
  try {
    // Load client
    const options = AnchorProvider.defaultOptions();
    const connection = new Connection(RPC_URL!, options);
    const user = Keypair.fromSecretKey(Buffer.from(JSON.parse(KEYPAIR)));
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
    const mangoAccount = await client.getMangoAccount(MANGO_ACCOUNT);
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
    // TODO: refactor so it can be called inside onExit
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

    // Initialize bot state
    const perpMarkets: Map<PerpMarketIndex, MarketContext> = new Map();
    for (const perpMarketAsset of getPerpMarketAssetsToTradeOn(group)) {
      const perpMarket = group.getPerpMarketByName(perpMarketAsset + '-PERP');
      const collateralBank = group.getFirstBankByMint(
        new PublicKey(params.assets[perpMarketAsset].perp.tokenMint),
      );
      const [sequenceAccount, sequenceAccountBump] =
        await findOwnSeqEnforcerAddress(perpMarket.name, client);
      perpMarkets.set(perpMarket.perpMarketIndex, {
        params: params.assets[perpMarketAsset].perp,
        perpMarket,
        collateralBank,
        sequenceAccount,
        sequenceAccountBump,
        unprocessedPerpFills: [],
      });
    }

    const bot = new BotContext(client, group, mangoAccount, perpMarkets);

    // Init sequence enforcer accounts if needed
    await initSequenceEnforcerAccounts(
      client,
      Array.from(perpMarkets.values()),
    );

    // TODO: subscribe to position change events:
    //     const fillsWs = new WebSocket('https://api.mngo.cloud/fills/v1');
    //     store events in unprocessedPerpFills together with the relevant slot
    //     prune unprocessed fill events < response.slot here as well to avoid race conditions

    // Add handler for e.g. CTRL+C
    // TODO, this keep registering more and more handlers, maybe not ideal
    process.on('SIGINT', function () {
      console.log('Caught keyboard interrupt. Canceling orders');
      control.isRunning = false;
      // TODO: to execute promise until end, add .then() call
      onExit(client, group, mangoAccount, Array.from(perpMarkets.values()));
    });

    console.log('Fetch state for the first time');
    await bot.refreshAll();

    // setup continuous refresh
    console.log('Refresh state', params.intervals);
    // intervals.push(
    //   setInterval(() =>
    //     bot.refreshBlockhash().then(() => console.debug('updated blockhash')),
    //   ),
    //   params.intervals.blockhash,
    // );
    // intervals.push(
    //   setInterval(() =>
    //     bot.refreshGroup().then(() => console.debug('updated group')),
    //   ),
    //   params.intervals.group,
    // );
    // intervals.push(
    //   setInterval(() =>
    //     bot
    //       .refreshMangoAccount()
    //       .then(() => console.debug('updated mangoAccount')),
    //   ),
    //   params.intervals.mangoAccount,
    // );

    // place initial oracle peg orders on book
    await Promise.all(
      Array.from(perpMarkets.keys()).map(async (i) => bot.updateOrders(i)),
    );

    // launch hedgers per market until control isRunning = false
    await Promise.all(
      Array.from(perpMarkets.keys()).map(async (i) => bot.hedgeFills(i)),
    );
  } finally {
    intervals.forEach((i) => clearInterval(i));
    intervals = [];
  }
}

function startMarketMaker(): void {
  try {
    if (control.isRunning) {
      console.warn('start MM');
      fullMarketMaker()
        .catch((e) => console.error('Critical Error', e))
        .finally(startMarketMaker);
    }
  } catch (e) {
    console.error('this should never happen', e);
  }
}

startMarketMaker();
