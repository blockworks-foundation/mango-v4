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
import { toUiDecimalsForQuote } from '../../src/utils';
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

  async updateOrders(i: PerpMarketIndex): Promise<string> {
    const { params, perpMarket } = this.perpMarkets.get(i)!;
    // TODO calculate current delta (spot + perp) & equity
    const currentDelta = 0;
    const currentEquityInUnderlying =
      this.mangoAccount.getEquity(this.group).toNumber() / p.perpMarket.uiPrice;
    const maxSize = currentEquityInUnderlying * params.sizePerc;
    const bidSize = Math.min(maxSize, params.deltaMax - currentDelta);
    const askSize = Math.min(maxSize, currentDelta - params.deltaMin);
    const bidPrice = perpMarket.uiPrice * -params.charge;
    const askPrice = perpMarket.uiPrice * +params.charge;

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
      beginIx,
      cancelAllIx,
      bidIx,
      askIx,
      endIx,
    ]);
  }

  async hedgeFills(i: PerpMarketIndex): Promise<void> {
    console.log('start hedger', i);
    while (control.isRunning) {
      // calculate delta
      //       hedge to bring delta in line with config goal
      //       if perp position would be reduced by hedge:
      //          hedge on perp with kill or fill
      //          spot hedge in parallel with same sequence id but a second tx
      //          if perp hedge fails, sequence id won't be advanced and spot hedge will succeed
      //          spot hedge should use known liquid routes without network requests (mango-router run in parallel)
      //       if perp position would increase:
      //          spot hedge
      //       wait for hedge txs to confirm:
      //          update oracle peg orders
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
  for (const mc of marketContexts) {
    // TODO: this doesn't actually enforce execution as promise is never awaited for
    cancelAllOrdersForAMarket(client, group, mangoAccount, mc.perpMarket);
  }
}

// Main driver for the market maker
async function fullMarketMaker() {
  let intervals: NodeJS.Timer[] = [];
  try {
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
    const mangoAccount = await client.getMangoAccount(
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
        new PublicKey(params.assets[perpMarketAsset].perp.collateralMint),
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

    console.log(`Refreshing state first time`);
    await bot.refreshAll();

    // setup continuous refresh
    intervals.push(
      setInterval(() =>
        this.refreshBlockhash.then(() => console.debug('updated blockhash')),
      ),
      params.intervals.blockhash,
    );
    intervals.push(
      setInterval(() =>
        this.refreshGroup.then(() => console.debug('updated group')),
      ),
      params.intervals.group,
    );
    intervals.push(
      setInterval(() =>
        this.refreshMangoAccount.then(() =>
          console.debug('updated mangoAccount'),
        ),
      ),
      params.intervals.mangoAccount,
    );

    // place initial oracle peg orders on book
    await Promise.all(
      Array.from(perpMarkets.keys()).map(async (i) => this.updateOrders(i)),
    );

    // launch hedgers per market until control isRunning = false
    await Promise.all(
      Array.from(perpMarkets.keys()).map(async (i) => this.hedgeFills(i)),
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
