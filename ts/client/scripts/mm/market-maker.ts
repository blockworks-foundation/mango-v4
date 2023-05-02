import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import {
  BlockhashWithExpiryBlockHeight,
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import console from 'console';
import fs from 'fs';
import path from 'path';
import WebSocket from 'ws';
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
import {
  QUOTE_DECIMALS,
  toNative,
  toUiDecimalsForQuote,
} from '../../src/utils';
import { sendTransaction } from '../../src/utils/rpc';
import * as defaultParams from './params/default.json';
import {
  fetchJupiterTransaction,
  fetchRoutes,
  prepareMangoRouterInstructions,
  RouteInfo,
} from './router';
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
const KEYPAIR = Keypair.fromSecretKey(
  Buffer.from(
    JSON.parse(
      process.env.KEYPAIR ||
        fs.readFileSync(process.env.KEYPAIR_PATH!, 'utf-8'),
    ),
  ),
);
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

const prec = (n: number): string => n.toFixed(params.logPrecision);

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
      console.error('could not refresh group', e);
    }
  }

  async refreshMangoAccount(): Promise<number | undefined> {
    if (!control.isRunning) return;
    try {
      const preserveOOs = this.mangoAccount.serum3OosMapByMarketIndex;
      const response = await this.client.getMangoAccountWithSlot(
        this.mangoAccount.publicKey,
      );
      if (response) {
        this.mangoAccount = response.value;
        this.mangoAccount.serum3OosMapByMarketIndex = preserveOOs;
        this.mangoAccountLastUpdatedSlot = response.slot;
        this.mangoAccountLastUpdatedTs = Date.now();
        return response.slot;
      } else {
        console.warn('could not fetch mangoAccount');
      }
    } catch (e) {
      console.error('could not refresh mangoAccount', e);
    }
  }

  calculateDelta(i: PerpMarketIndex) {
    const { params, perpMarket, unprocessedPerpFills, eventQueueHeadBySlot } =
      this.perpMarkets.get(i)!;

    const tokenMint = new PublicKey(params.tokenMint);
    const tokenBank = this.group.getFirstBankByMint(tokenMint);
    const tokenPosition = this.mangoAccount.getTokenBalanceUi(tokenBank);
    let perpPosition = 0;
    try {
      perpPosition = this.mangoAccount.getPerpPositionUi(
        this.group,
        perpMarket.perpMarketIndex,
      );
      // eslint-disable-next-line no-empty
    } catch (_) {}


    let fillsSinceLastUpdate: FillEventUpdate[] = []
    let eventQueueHeadUpdatesBeforeLastMangoAccountUpdate = eventQueueHeadBySlot.filter(e => e.slot <= this.mangoAccountLastUpdatedSlot);
    if (eventQueueHeadUpdatesBeforeLastMangoAccountUpdate.length > 0)
    {
      let eventQueueHeadAtMangoAccountUpdate = eventQueueHeadUpdatesBeforeLastMangoAccountUpdate[eventQueueHeadUpdatesBeforeLastMangoAccountUpdate.length-1];
      fillsSinceLastUpdate = unprocessedPerpFills.filter(
        (f) => f.event.seqNum >= eventQueueHeadAtMangoAccountUpdate.head
      );
    }

    const fillsDelta = fillsSinceLastUpdate.reduce((d, u) => {
      const isMaker = u.event.maker == MANGO_ACCOUNT.toString();
      const isNew = u.status == 'new';
      const takerBids = u.event.takerSide == 'bid';

      // isMaker = true, isNew = true, takerBids = true => reduce delta
      // if one of those flips, we need to increase delta
      // if two flip, we reduce
      // if all flip we increase
      // us negated equality for xor
      const deltaSign = (isMaker !== isNew) !== takerBids ? -1 : 1;

      return d + deltaSign * u.event.quantity;
    }, 0);


    // TODO: add fill delta into perpPosition
    return {
      delta: tokenPosition + perpPosition + fillsDelta,
      tokenPosition,
      perpPosition,
      fillsDelta,
    };
  }

  async updateOrders(i: PerpMarketIndex): Promise<string> {
    const { params, perpMarket } = this.perpMarkets.get(i)!;
    const { delta, perpPosition, fillsDelta } = this.calculateDelta(i);
    const currentEquityInUnderlying =
      toUiDecimalsForQuote(this.mangoAccount.getEquity(this.group).toNumber()) /
      perpMarket.uiPrice;
    const maxSize = currentEquityInUnderlying * params.orderSizeLimit;
    const bidSize = Math.min(
      maxSize,
      params.positionMax - perpPosition, // dont buy if position > positionMax
    );
    const askSize = Math.min(
      maxSize,
      perpPosition + params.positionMax, // don't sell if position < - positionMax
    );
    const bidPrice = perpMarket.uiPrice * -params.spreadCharge;
    const askPrice = perpMarket.uiPrice * +params.spreadCharge;

    console.log(
      `update orders ${perpMarket.name} delta=${prec(delta)} fills=${prec(fillsDelta)} equity=${prec(
        currentEquityInUnderlying,
      )} mark=${prec(perpMarket.uiPrice)} bid=${prec(bidSize)}@${prec(
        bidPrice,
      )} ask=${prec(askSize)}@${prec(askPrice)}`,
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

    return this.client.sendAndConfirmTransaction(
      [
        // beginIx,
        cancelAllIx,
        bidSize > 0 ? bidIx : null,
        askSize > 0 ? askIx : null,
        // endIx,
      ].filter((i) => !!i) as TransactionInstruction[],
    );
  }

  async hedgeFills(i: PerpMarketIndex): Promise<void> {
    const { params, perpMarket } = this.perpMarkets.get(i)!;

    const tokenMint = new PublicKey(params.tokenMint);

    console.log('start hedger', i, perpMarket.name);
    while (control.isRunning) {
      const { delta, perpPosition, tokenPosition, fillsDelta } =
        this.calculateDelta(i);
      const biasedDelta = delta + params.deltaBias;

      // console.log(
      //   `hedge check ${perpMarket.name} d=${delta.toFixed(
      //     3,
      //   )} t=${tokenPosition.toFixed(3)} p=${perpPosition.toFixed(
      //     3,
      //   )} f=${fillsDelta.toFixed(3)}`,
      // );

      if (Math.abs(biasedDelta) > perpMarket.minOrderSize) {
        // prefer perp hedge if perp position has same sign as account delta
        /* if (Math.sign(biasedDelta) * Math.sign(perpPosition) > 0) {
          // hedge size is limited to split hedges into reasonable order sizes
          const hedgeSize = Math.min(Math.abs(biasedDelta), params.hedgeMax);
          // hedge price needs to cross the spread and offer a discount
          const hedgePrice =
            perpMarket.uiPrice *
            (1 - Math.sign(biasedDelta) * params.hedgeDiscount);

          const side =
            Math.sign(biasedDelta) > 0 ? PerpOrderSide.ask : PerpOrderSide.bid;

          console.log(
            `hedge perp delta=${biasedDelta}/${perpPosition} side=${
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

          await this.updateOrders(i);
        } else */ {
          // hedge on spot
          // hedge size is limited to split hedges into reasonable order sizes
          const hedgeSize = Math.min(Math.abs(biasedDelta), params.hedgeMax);
          // hedge price needs to cross the spread and offer a discount
          const hedgePrice =
            perpMarket.uiPrice *
            (1 - Math.sign(biasedDelta) * params.hedgeDiscount);

          const inputMint =
            Math.sign(biasedDelta) > 0
              ? tokenMint
              : this.group.getFirstBankForPerpSettlement().mint;
          const outputMint =
            Math.sign(biasedDelta) > 0
              ? this.group.getFirstBankForPerpSettlement().mint
              : tokenMint;
          const swapMode = Math.sign(biasedDelta) > 0 ? 'ExactIn' : 'ExactOut';
          const amount = toNative(
            hedgeSize,
            this.group.getFirstBankByMint(tokenMint).mintDecimals,
          );

          console.log(
            `hedge ${perpMarket.name} on spot delta=${prec(biasedDelta)} fills=${prec(
              fillsDelta,
            )} inputMint=${inputMint.toString()} outputMint=${outputMint.toString()} size=${prec(
              hedgeSize,
            )} limit=${prec(hedgePrice)} index=${prec(perpMarket.uiPrice)}`,
          );

          const { bestRoute } = await fetchRoutes(
            inputMint,
            outputMint,
            amount.toString(),
            params.hedgeDiscount,
            swapMode,
            '0',
            KEYPAIR.publicKey,
          );

          if (!bestRoute) {
            console.error(
              `${perpMarket.name} spot hedge could not find a route`,
            );
          } else {
            const [ixs, alts] =
              bestRoute.routerName === 'Mango'
                ? await prepareMangoRouterInstructions(
                    bestRoute,
                    inputMint,
                    outputMint,
                    KEYPAIR.publicKey,
                  )
                : await fetchJupiterTransaction(
                    this.client.connection,
                    bestRoute,
                    KEYPAIR.publicKey,
                    params.hedgeDiscount,
                    inputMint,
                    outputMint,
                  );

            try {
              const confirmBegin = Date.now();
              const sig = await this.client.marginTrade({
                group: this.group,
                mangoAccount: this.mangoAccount,
                inputMintPk: inputMint,
                amountIn:
                  Math.sign(biasedDelta) > 0
                    ? hedgeSize
                    : bestRoute.inAmount /
                      Math.pow(
                        10,
                        this.group.getFirstBankForPerpSettlement().mintDecimals,
                      ),
                outputMintPk: outputMint,
                userDefinedInstructions: ixs,
                userDefinedAlts: alts,
                flashLoanType: { swap: {} },
              });
              const confirmEnd = Date.now();

              await this.refreshMangoAccount();

              const newDelta = this.calculateDelta(i).delta;

              console.log(
                `hedge ${i} ${perpMarket.name} confirmed delta time=${
                  (confirmEnd - confirmBegin) / 1000
                } prev=${prec(delta)} new=${prec(
                  newDelta,
                )} https://explorer.solana.com/tx/${sig}`,
              );

              await this.updateOrders(i);
            } catch (e) {
              console.error(perpMarket.name, 'spot hedge failed', e);
            }
          }
        }
      }
      await sleep(10); // sleep a few ms
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
  unprocessedPerpFills: FillEventUpdate[];
  eventQueueHeadBySlot: {slot: number, head: number}[];
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
  process.exit(-1);
}

function sleep(ms): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

interface FillEventUpdate {
  status: 'new' | 'revoke';
  marketKey: 'string';
  marketName: 'string';
  slot: number;
  writeVersion: number;
  event: {
    eventType: 'spot' | 'perp';
    maker: 'string';
    taker: 'string';
    takerSide: 'bid' | 'ask';
    timestamp: 'string'; // DateTime
    seqNum: number;
    makerClientOrderId: number;
    takerClientOrderId: number;
    makerFee: number;
    takerFee: number;
    price: number;
    quantity: number;
  };
}

function isFillEventUpdate(obj: any): obj is FillEventUpdate {
  return obj.event !== undefined;
}

interface HeadUpdate {
  head: number;
  previousHead: number;
  headSeqNum: number;
  previousHeadSeqNum: number;
  status: 'new' | 'revoke';
  marketKey: 'string';
  marketName: 'string';
  slot: number;
  writeVersion: number;
}

function isHeadUpdate(obj: any): obj is HeadUpdate {
  return obj.head !== undefined;
}

// Main driver for the market maker
async function fullMarketMaker(): Promise<void> {
  let intervals: NodeJS.Timer[] = [];
  let fillsWs: WebSocket | undefined;
  try {
    // Load client
    const options = AnchorProvider.defaultOptions();
    options.commitment = 'processed';
    const connection = new Connection(RPC_URL!, options);
    const userWallet = new Wallet(KEYPAIR);
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
      `MangoAccount ${
        mangoAccount.publicKey
      } of owner ${mangoAccount.owner.toString()} ${
        mangoAccount.isDelegate(client)
          ? 'via delegate ' + KEYPAIR.publicKey
          : ''
      }`,
    );
    // reload mango account to load the serum3 oos
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
        eventQueueHeadBySlot: [],
      });
    }

    const bot = new BotContext(client, group, mangoAccount, perpMarkets);

    // Init sequence enforcer accounts if needed
    await initSequenceEnforcerAccounts(
      client,
      Array.from(perpMarkets.values()),
    );

    // TODO: subscribe to position change events:
    fillsWs = new WebSocket('wss://api.mngo.cloud/fills/v1/');
    fillsWs.addEventListener('open', (_) => {
      for (const [i, pc] of perpMarkets.entries()) {
        fillsWs!.send(
          JSON.stringify({
            command: 'subscribe',
            marketId: pc.perpMarket.publicKey.toBase58(),
            headUpdates: true,
          }),
          // eslint-disable-next-line @typescript-eslint/no-empty-function
          (_) => {
            console.log(
              `fills websocket subscribed ${pc.perpMarket.name} ${_}`,
            );
          },
        );
      }
      console.log('fills websocket open');
    });
    // Listen for messages
    fillsWs.addEventListener('message', (msg) => {
      const data = JSON.parse(msg.data);

      // fill added to queue
      if (isFillEventUpdate(data)) {
        const eventMarket = new PublicKey(data.marketKey);
        for (const pc of perpMarkets.values()) {
          if (!pc.perpMarket.publicKey.equals(eventMarket)) continue;
          // check if maker xor taker equals to mango account to filter out irrelevant or self-trades
          if (
            (data.event.maker == MANGO_ACCOUNT.toString()) !==
            (data.event.taker == MANGO_ACCOUNT.toString())
          ) {
            console.log(
              `fill id=${data.event.seqNum} status=${data.status} slot=${data.slot} size=${data.event.quantity} price=${data.event.price} taker=${data.event.taker.slice(0, 4)} maker=${data.event.maker.slice(0, 4)} takerSide=${data.event.takerSide}`
            );

            // prune unprocessed fill events before last head update here to avoid race conditions
            let eventQueueHeadUpdatesBeforeLastMangoAccountUpdate = pc.eventQueueHeadBySlot.filter(e => e.slot <= bot.mangoAccountLastUpdatedSlot);
            if (eventQueueHeadUpdatesBeforeLastMangoAccountUpdate.length > 0)
            {
              let eventQueueHeadAtMangoAccountUpdate = eventQueueHeadUpdatesBeforeLastMangoAccountUpdate[eventQueueHeadUpdatesBeforeLastMangoAccountUpdate.length-1];
              // truncate list to a reasonable limit
              let deleteCount = Math.max(0, pc.unprocessedPerpFills.length - 500);
              pc.unprocessedPerpFills.splice(0, deleteCount);
            }
            pc.unprocessedPerpFills.push(data);
            break;
          }
        }
      }

      // events consumed
      if (isHeadUpdate(data)) {
        for (const pc of perpMarkets.values()) {
          if (!pc.perpMarket.publicKey.equals(new PublicKey(data.marketKey))) continue;
          console.log('received HeadUpdate', data);
          // truncate list to a reasonable limit
          let deleteCount = Math.max(0, pc.eventQueueHeadBySlot.length - 500);
          pc.eventQueueHeadBySlot.splice(0, deleteCount);
          pc.eventQueueHeadBySlot.push({slot: data.slot, head: data.headSeqNum});
        }
      }
    });

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
    await mangoAccount.reloadSerum3OpenOrders(client);

    // setup continuous refresh
    console.log('Refresh state', params.intervals);
    intervals.push(
      setInterval(
        () =>
          bot.refreshBlockhash().then(() => console.log('updated blockhash')),
        params.intervals.blockhash,
      ),
    );
    intervals.push(
      setInterval(
        () => bot.refreshGroup().then(() => console.debug('updated group')),
        params.intervals.group,
      ),
    );
    intervals.push(
      setInterval(
        () =>
          bot
            .refreshMangoAccount()
            .then((s) => console.debug('updated mangoAccount', s)),
        params.intervals.mangoAccount,
      ),
    );

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
    if (fillsWs) fillsWs.close();
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
function fetchMangoRoutes(
  inputMint: PublicKey,
  outputMint: PublicKey,
  amount: BN,
  hedgeDiscount: any,
  swapMode: string,
  arg5: number,
  publicKey: PublicKey,
): any {
  throw new Error('Function not implemented.');
}

function fetchJupiterRoutes(
  inputMint: PublicKey,
  outputMint: PublicKey,
  amount: BN,
  hedgeDiscount: any,
  swapMode: string,
  arg5: number,
): any {
  throw new Error('Function not implemented.');
}

