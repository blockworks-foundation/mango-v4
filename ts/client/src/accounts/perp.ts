import { BN } from '@coral-xyz/anchor';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import Big from 'big.js';
import { MangoClient } from '../client';
import { RUST_U64_MAX } from '../constants';
import { I80F48, I80F48Dto, ZERO_I80F48 } from '../numbers/I80F48';
import { Modify } from '../types';
import {
  As,
  QUOTE_DECIMALS,
  U64_MAX_BN,
  toNative,
  toUiDecimals,
} from '../utils';
import {
  OracleConfig,
  OracleConfigDto,
  StablePriceModel,
  TokenIndex,
} from './bank';
import { Group } from './group';
import { MangoAccount } from './mangoAccount';
import { OracleProvider, isOracleStaleOrUnconfident } from './oracle';

export type PerpMarketIndex = number & As<'perp-market-index'>;

export type ParsedFillEvent = Modify<
  FillEvent,
  {
    price: number;
    quantity: number;
  }
>;

export class PerpMarket {
  public name: string;
  public oracleConfig: OracleConfig;
  public maintBaseAssetWeight: I80F48;
  public initBaseAssetWeight: I80F48;
  public maintBaseLiabWeight: I80F48;
  public initBaseLiabWeight: I80F48;
  public baseLiquidationFee: I80F48;
  public makerFee: I80F48;
  public takerFee: I80F48;
  public minFunding: I80F48;
  public maxFunding: I80F48;
  public longFunding: I80F48;
  public shortFunding: I80F48;
  public feesAccrued: I80F48;
  public feesSettled: I80F48;
  public maintOverallAssetWeight: I80F48;
  public initOverallAssetWeight: I80F48;
  public positivePnlLiquidationFee: I80F48;
  public platformLiquidationFee: I80F48;
  public accruedLiquidationFees: I80F48;

  public _price: I80F48;
  public _uiPrice: number;
  public _oracleLastUpdatedSlot: number;
  public _oracleLastKnownDeviation: I80F48 | undefined;
  public _oracleProvider: OracleProvider;

  public _bids: BookSide;
  public _asks: BookSide;

  private priceLotsToUiConverter: number;
  private baseLotsToUiConverter: number;
  private quoteLotsToUiConverter: number;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      settleTokenIndex: number;
      perpMarketIndex: number;
      groupInsuranceFund: number;
      baseDecimals: number;
      name: number[];
      bids: PublicKey;
      asks: PublicKey;
      eventQueue: PublicKey;
      oracle: PublicKey;
      oracleConfig: OracleConfigDto;
      stablePriceModel: StablePriceModel;
      quoteLotSize: BN;
      baseLotSize: BN;
      maintBaseAssetWeight: I80F48Dto;
      initBaseAssetWeight: I80F48Dto;
      maintBaseLiabWeight: I80F48Dto;
      initBaseLiabWeight: I80F48Dto;
      openInterest: BN;
      seqNum: BN;
      registrationTime: BN;
      minFunding: I80F48Dto;
      maxFunding: I80F48Dto;
      impactQuantity: BN;
      longFunding: I80F48Dto;
      shortFunding: I80F48Dto;
      fundingLastUpdated: BN;
      baseLiquidationFee: I80F48Dto;
      makerFee: I80F48Dto;
      takerFee: I80F48Dto;
      feesAccrued: I80F48Dto;
      feesSettled: I80F48Dto;
      feePenalty: number;
      settleFeeFlat: number;
      settleFeeAmountThreshold: number;
      settleFeeFractionLowHealth: number;
      settlePnlLimitFactor: number;
      settlePnlLimitWindowSizeTs: BN;
      reduceOnly: number;
      forceClose: number;
      maintOverallAssetWeight: I80F48Dto;
      initOverallAssetWeight: I80F48Dto;
      positivePnlLiquidationFee: I80F48Dto;
      feesWithdrawn: BN;
      platformLiquidationFee: I80F48Dto;
      accruedLiquidationFees: I80F48Dto;
    },
  ): PerpMarket {
    return new PerpMarket(
      publicKey,
      obj.group,
      obj.settleTokenIndex as TokenIndex,
      obj.perpMarketIndex as PerpMarketIndex,
      obj.groupInsuranceFund == 1,
      obj.baseDecimals,
      obj.name,
      obj.bids,
      obj.asks,
      obj.eventQueue,
      obj.oracle,
      obj.oracleConfig,
      obj.stablePriceModel,
      obj.quoteLotSize,
      obj.baseLotSize,
      obj.maintBaseAssetWeight,
      obj.initBaseAssetWeight,
      obj.maintBaseLiabWeight,
      obj.initBaseLiabWeight,
      obj.openInterest,
      obj.seqNum,
      obj.registrationTime,
      obj.minFunding,
      obj.maxFunding,
      obj.impactQuantity,
      obj.longFunding,
      obj.shortFunding,
      obj.fundingLastUpdated,
      obj.baseLiquidationFee,
      obj.makerFee,
      obj.takerFee,
      obj.feesAccrued,
      obj.feesSettled,
      obj.feePenalty,
      obj.settleFeeFlat,
      obj.settleFeeAmountThreshold,
      obj.settleFeeFractionLowHealth,
      obj.settlePnlLimitFactor,
      obj.settlePnlLimitWindowSizeTs,
      obj.reduceOnly == 1,
      obj.forceClose == 1,
      obj.maintOverallAssetWeight,
      obj.initOverallAssetWeight,
      obj.positivePnlLiquidationFee,
      obj.feesWithdrawn,
      obj.platformLiquidationFee,
      obj.accruedLiquidationFees,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public settleTokenIndex: TokenIndex,
    public perpMarketIndex: PerpMarketIndex, // TODO rename to marketIndex?
    public groupInsuranceFund: boolean,
    public baseDecimals: number,
    name: number[],
    public bids: PublicKey,
    public asks: PublicKey,
    public eventQueue: PublicKey,
    public oracle: PublicKey,
    oracleConfig: OracleConfigDto,
    public stablePriceModel: StablePriceModel,
    public quoteLotSize: BN,
    public baseLotSize: BN,
    maintBaseAssetWeight: I80F48Dto,
    initBaseAssetWeight: I80F48Dto,
    maintBaseLiabWeight: I80F48Dto,
    initBaseLiabWeight: I80F48Dto,
    public openInterest: BN,
    public seqNum: BN,
    public registrationTime: BN,
    minFunding: I80F48Dto,
    maxFunding: I80F48Dto,
    public impactQuantity: BN,
    longFunding: I80F48Dto,
    shortFunding: I80F48Dto,
    public fundingLastUpdated: BN,
    baseLiquidationFee: I80F48Dto,
    makerFee: I80F48Dto,
    takerFee: I80F48Dto,
    feesAccrued: I80F48Dto,
    feesSettled: I80F48Dto,
    public feePenalty: number,
    public settleFeeFlat: number,
    public settleFeeAmountThreshold: number,
    public settleFeeFractionLowHealth: number,
    public settlePnlLimitFactor: number,
    public settlePnlLimitWindowSizeTs: BN,
    public reduceOnly: boolean,
    public forceClose: boolean,
    maintOverallAssetWeight: I80F48Dto,
    initOverallAssetWeight: I80F48Dto,
    positivePnlLiquidationFee: I80F48Dto,
    public feesWithdrawn: BN,
    platformLiquidationFee: I80F48Dto,
    accruedLiquidationFees: I80F48Dto,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.oracleConfig = {
      confFilter: I80F48.from(oracleConfig.confFilter),
      maxStalenessSlots: oracleConfig.maxStalenessSlots,
    } as OracleConfig;
    this.maintBaseAssetWeight = I80F48.from(maintBaseAssetWeight);
    this.initBaseAssetWeight = I80F48.from(initBaseAssetWeight);
    this.maintBaseLiabWeight = I80F48.from(maintBaseLiabWeight);
    this.initBaseLiabWeight = I80F48.from(initBaseLiabWeight);
    this.baseLiquidationFee = I80F48.from(baseLiquidationFee);
    this.makerFee = I80F48.from(makerFee);
    this.takerFee = I80F48.from(takerFee);
    this.minFunding = I80F48.from(minFunding);
    this.maxFunding = I80F48.from(maxFunding);
    this.longFunding = I80F48.from(longFunding);
    this.shortFunding = I80F48.from(shortFunding);
    this.feesAccrued = I80F48.from(feesAccrued);
    this.feesSettled = I80F48.from(feesSettled);
    this.maintOverallAssetWeight = I80F48.from(maintOverallAssetWeight);
    this.initOverallAssetWeight = I80F48.from(initOverallAssetWeight);
    this.positivePnlLiquidationFee = I80F48.from(positivePnlLiquidationFee);
    this.platformLiquidationFee = I80F48.from(platformLiquidationFee);
    this.accruedLiquidationFees = I80F48.from(accruedLiquidationFees);

    this.priceLotsToUiConverter = new Big(10)
      .pow(baseDecimals - QUOTE_DECIMALS)
      .mul(new Big(this.quoteLotSize.toString()))
      .div(new Big(this.baseLotSize.toString()))
      .toNumber();

    this.baseLotsToUiConverter = new Big(this.baseLotSize.toString())
      .div(new Big(10).pow(baseDecimals))
      .toNumber();

    this.quoteLotsToUiConverter = new Big(this.quoteLotSize.toString())
      .div(new Big(10).pow(QUOTE_DECIMALS))
      .toNumber();
  }

  isOracleStaleOrUnconfident(nowSlot: number): boolean {
    return isOracleStaleOrUnconfident(
      nowSlot,
      this.oracleConfig.maxStalenessSlots.toNumber(),
      this.oracleLastUpdatedSlot,
      this._oracleLastKnownDeviation,
      this.oracleConfig.confFilter,
      this.price,
    );
  }

  get price(): I80F48 {
    if (this._price === undefined) {
      throw new Error(
        `Undefined price for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`,
      );
    }
    return this._price;
  }

  get uiPrice(): number {
    if (this._uiPrice === undefined) {
      throw new Error(
        `Undefined price for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`,
      );
    }
    return this._uiPrice;
  }

  get oracleLastUpdatedSlot(): number {
    if (this._oracleLastUpdatedSlot === undefined) {
      throw new Error(
        `Undefined oracleLastUpdatedSlot for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`,
      );
    }
    return this._oracleLastUpdatedSlot;
  }

  get oracleProvider(): OracleProvider {
    if (this._oracleProvider === undefined) {
      throw new Error(
        `Undefined oracleProvider for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`,
      );
    }
    return this._oracleProvider;
  }

  get minOrderSize(): number {
    return this.baseLotsToUiConverter;
  }

  get tickSize(): number {
    return this.priceLotsToUiConverter;
  }

  insidePriceLimit(side: PerpOrderSide, orderPrice: number): boolean {
    return (
      (side === PerpOrderSide.bid &&
        orderPrice <= this.maintBaseLiabWeight.toNumber() * this.uiPrice) ||
      (side === PerpOrderSide.ask &&
        orderPrice >= this.maintBaseAssetWeight.toNumber() * this.uiPrice)
    );
  }

  public async loadAsks(
    client: MangoClient,
    forceReload = false,
  ): Promise<BookSide> {
    if (forceReload || !this._asks) {
      const asks = await client.program.account.bookSide.fetch(this.asks);
      this._asks = BookSide.from(client, this, BookSideType.asks, asks as any);
    }
    return this._asks;
  }

  public async loadBids(
    client: MangoClient,
    forceReload = false,
  ): Promise<BookSide> {
    if (forceReload || !this._bids) {
      const bids = await client.program.account.bookSide.fetch(this.bids);
      this._bids = BookSide.from(client, this, BookSideType.bids, bids as any);
    }
    return this._bids;
  }

  public async loadEventQueue(client: MangoClient): Promise<PerpEventQueue> {
    const eventQueue = await client.program.account.eventQueue.fetch(
      this.eventQueue,
    );
    return new PerpEventQueue(client, eventQueue.header, eventQueue.buf);
  }

  public async loadFills(
    client: MangoClient,
    lastSeqNum: BN = new BN(0),
  ): Promise<FillEvent[]> {
    const eventQueue = await this.loadEventQueue(client);
    return eventQueue
      .eventsSince(lastSeqNum)
      .filter((event) => event.eventType == PerpEventQueue.FILL_EVENT_TYPE)
      .map(this.parseFillEvent.bind(this)) as ParsedFillEvent[];
  }

  public parseFillEvent(event): ParsedFillEvent {
    const quantity = this.baseLotsToUi(event.quantity);
    const price = this.priceLotsToUi(event.price);

    return {
      ...event,
      quantity,
      size: quantity,
      price,
    };
  }

  public async logOb(client: MangoClient): Promise<string> {
    let res = ``;
    res += `  ${this.name} OrderBook`;
    let orders = await this?.loadAsks(client);
    for (const order of orders!.items()) {
      res += `\n ${order.uiPrice.toFixed(5).padStart(10)}, ${order.uiSize
        .toString()
        .padStart(10)} ${
        order.isOraclePegged && order.oraclePeggedProperties
          ? order.oraclePeggedProperties.pegLimit.toNumber() + ' (PegLimit)'
          : ''
      }`;
    }
    res += `\n  asks ↑ --------- ↓ bids`;
    orders = await this?.loadBids(client);
    for (const order of orders!.items()) {
      res += `\n  ${order.uiPrice.toFixed(5).padStart(10)}, ${order.uiSize
        .toString()
        .padStart(10)} ${
        order.isOraclePegged && order.oraclePeggedProperties
          ? order.oraclePeggedProperties.pegLimit.toNumber() + ' (PegLimit)'
          : ''
      }`;
    }
    return res;
  }

  /**
   *
   * @param bids
   * @param asks
   * @returns returns instantaneous funding rate
   */
  public getInstantaneousFundingRate(bids: BookSide, asks: BookSide): number {
    const MIN_FUNDING = this.minFunding.toNumber();
    const MAX_FUNDING = this.maxFunding.toNumber();

    const bid = bids.getImpactPriceUi(new BN(this.impactQuantity));
    const ask = asks.getImpactPriceUi(new BN(this.impactQuantity));
    const indexPrice = this._uiPrice;

    let funding;
    if (bid !== undefined && ask !== undefined) {
      const bookPrice = (bid + ask) / 2;
      funding = Math.min(
        Math.max(bookPrice / indexPrice - 1, MIN_FUNDING),
        MAX_FUNDING,
      );
    } else if (bid !== undefined) {
      funding = MAX_FUNDING;
    } else if (ask !== undefined) {
      funding = MIN_FUNDING;
    } else {
      funding = 0;
    }

    return funding;
  }

  public getInstantaneousFundingRatePerSecond(
    bids: BookSide,
    asks: BookSide,
  ): number {
    return this.getInstantaneousFundingRate(bids, asks) / (24 * 60 * 60);
  }

  /**
   *
   * Returns instantaneous funding rate for the day. How is it actually applied - funding is
   * continuously applied on every interaction to a perp position. The rate is further multiplied
   * by the time elapsed since it was last applied (capped to max. 1hr).
   *
   * @param bids
   * @param asks
   * @returns returns instantaneous funding rate in % form
   */
  public getInstantaneousFundingRateUi(bids: BookSide, asks: BookSide): number {
    return this.getInstantaneousFundingRate(bids, asks) * 100;
  }

  public uiPriceToLots(price: number): BN {
    return toNative(price, QUOTE_DECIMALS)
      .mul(this.baseLotSize)
      .div(this.quoteLotSize.mul(new BN(Math.pow(10, this.baseDecimals))));
  }

  public uiBaseToLots(quantity: number): BN {
    return toNative(quantity, this.baseDecimals).div(this.baseLotSize);
  }

  public uiQuoteToLots(uiQuote: number): BN {
    return toNative(uiQuote, QUOTE_DECIMALS).div(this.quoteLotSize);
  }

  public priceLotsToNative(price: BN): I80F48 {
    return I80F48.fromI64(price.mul(this.quoteLotSize).div(this.baseLotSize));
  }

  public priceLotsToUi(price: BN): number {
    return parseFloat(price.toString()) * this.priceLotsToUiConverter;
  }

  public priceNativeToUi(price: number): number {
    return toUiDecimals(price, QUOTE_DECIMALS - this.baseDecimals);
  }

  public baseLotsToUi(quantity: BN): number {
    return parseFloat(quantity.toString()) * this.baseLotsToUiConverter;
  }

  public quoteLotsToUi(quantity: BN): number {
    return parseFloat(quantity.toString()) * this.quoteLotsToUiConverter;
  }

  /**
   * Returns a list of (upto count) accounts, and the pnl that is settle'able on this perp market,
   * the list is sorted ascending for 'negative' direction and descending for 'positive' direction.
   *
   * NOTE: keep in sync with perp_pnl.rs:fetch_top
   *
   * TODO: replace with a more performant offchain service call
   * @param client
   * @param group
   * @param direction
   * @returns
   */
  public async getSettlePnlCandidates(
    client: MangoClient,
    group: Group,
    accounts?: MangoAccount[],
    direction: 'negative' | 'positive' = 'positive',
    count = 2,
  ): Promise<{ account: MangoAccount; settleablePnl: I80F48 }[]> {
    let accountsWithSettleablePnl = (
      accounts ?? (await client.getAllMangoAccounts(group, true))
    )
      .filter((acc) => acc.perpPositionExistsForMarket(this))
      .map((acc) => {
        const pp = acc
          .perpActive()
          .find((pp) => pp.marketIndex === this.perpMarketIndex)!;

        return {
          account: acc,
          settleablePnl: pp.getSettleablePnl(group, this, acc),
        };
      });

    accountsWithSettleablePnl = accountsWithSettleablePnl
      .filter(
        (acc) =>
          // need perp positions with -ve pnl to settle +ve pnl and vice versa
          (direction === 'negative' && acc.settleablePnl.lt(ZERO_I80F48())) ||
          (direction === 'positive' && acc.settleablePnl.gt(ZERO_I80F48())),
      )
      .sort((a, b) =>
        direction === 'negative'
          ? // most negative
            a.settleablePnl.cmp(b.settleablePnl)
          : // most positive
            b.settleablePnl.cmp(a.settleablePnl),
      );

    if (direction === 'negative') {
      let stable = 0;
      for (let i = 0; i < accountsWithSettleablePnl.length; i++) {
        const acc = accountsWithSettleablePnl[i];
        const nextPnl =
          i + 1 < accountsWithSettleablePnl.length
            ? accountsWithSettleablePnl[i + 1].settleablePnl
            : ZERO_I80F48();

        const perpMaxSettle = acc.account.perpMaxSettle(
          group,
          this.settleTokenIndex,
        );
        acc.settleablePnl =
          // need positive settle health to settle against +ve pnl
          perpMaxSettle.gt(ZERO_I80F48())
            ? // can only settle min
              acc.settleablePnl.max(perpMaxSettle.neg())
            : ZERO_I80F48();

        // If the ordering was unchanged `count` times we know we have the top `count` accounts
        if (acc.settleablePnl.lte(nextPnl)) {
          stable += 1;
          if (stable >= count) {
            break;
          }
        }
      }
    }

    accountsWithSettleablePnl.sort((a, b) =>
      direction === 'negative'
        ? // most negative
          a.settleablePnl.cmp(b.settleablePnl)
        : // most positive
          b.settleablePnl.cmp(a.settleablePnl),
    );

    return accountsWithSettleablePnl.slice(0, count);
  }

  toString(): string {
    return (
      'PerpMarket ' +
      '\n perpMarketIndex -' +
      this.perpMarketIndex +
      '\n maintAssetWeight -' +
      this.maintBaseAssetWeight.toString() +
      '\n initAssetWeight -' +
      this.initBaseAssetWeight.toString() +
      '\n maintLiabWeight -' +
      this.maintBaseLiabWeight.toString() +
      '\n initLiabWeight -' +
      this.initBaseLiabWeight.toString() +
      '\n baseLiquidationFee -' +
      this.baseLiquidationFee.toString() +
      '\n makerFee -' +
      this.makerFee.toString() +
      '\n takerFee -' +
      this.takerFee.toString()
    );
  }
}

interface OrderTreeNodes {
  bumpIndex: number;
  freeListLen: number;
  freeListHead: number;
  nodes: [any];
}

interface OrderTreeRoot {
  maybeNode: number;
  leafCount: number;
}

export class BookSide {
  private static INNER_NODE_TAG = 1;
  private static LEAF_NODE_TAG = 2;
  now: BN;

  static from(
    client: MangoClient,
    perpMarket: PerpMarket,
    bookSideType: BookSideType,
    obj: {
      roots: OrderTreeRoot[];
      nodes: OrderTreeNodes;
    },
  ): BookSide {
    return new BookSide(
      client,
      perpMarket,
      bookSideType,
      obj.roots[0],
      obj.roots[1],
      obj.nodes,
    );
  }

  constructor(
    public client: MangoClient,
    public perpMarket: PerpMarket,
    public type: BookSideType,
    public rootFixed: OrderTreeRoot,
    public rootOraclePegged: OrderTreeRoot,
    public orderTreeNodes: OrderTreeNodes,
    maxBookDelay?: number,
  ) {
    // Determine the maxTimestamp found on the book to use for tif
    // If maxBookDelay is not provided, use 3600 as a very large number
    maxBookDelay = maxBookDelay === undefined ? 3600 : maxBookDelay;
    let maxTimestamp = new BN(new Date().getTime() / 1000 - maxBookDelay);
    for (const node of this.orderTreeNodes.nodes) {
      if (node.tag !== BookSide.LEAF_NODE_TAG) {
        continue;
      }

      const leafNode = BookSide.toLeafNode(client, node.data);
      if (leafNode.timestamp.gt(maxTimestamp)) {
        maxTimestamp = leafNode.timestamp;
      }
    }
    this.now = maxTimestamp;
  }

  static getPriceFromKey(key: BN): BN {
    return key.ushrn(64);
  }

  /**
   * iterates over all orders
   */
  public *items(): Generator<PerpOrder> {
    function isBetter(type: BookSideType, a: PerpOrder, b: PerpOrder): boolean {
      return a.priceLots.eq(b.priceLots)
        ? a.seqNum.lt(b.seqNum) // if prices are equal prefer perp orders in the order they are placed
        : type === BookSideType.bids // else compare the actual prices
        ? a.priceLots.gt(b.priceLots)
        : b.priceLots.gt(a.priceLots);
    }

    const fGen = this.fixedItems();
    const oPegGen = this.oraclePeggedItems();

    let fOrderRes = fGen.next();
    let oPegOrderRes = oPegGen.next();

    while (true) {
      if (fOrderRes.value && oPegOrderRes.value) {
        if (isBetter(this.type, fOrderRes.value, oPegOrderRes.value)) {
          yield fOrderRes.value;
          fOrderRes = fGen.next();
        } else {
          yield oPegOrderRes.value;
          oPegOrderRes = oPegGen.next();
        }
      } else if (fOrderRes.value && !oPegOrderRes.value) {
        yield fOrderRes.value;
        fOrderRes = fGen.next();
      } else if (!fOrderRes.value && oPegOrderRes.value) {
        yield oPegOrderRes.value;
        oPegOrderRes = oPegGen.next();
      } else if (!fOrderRes.value && !oPegOrderRes.value) {
        break;
      }
    }
  }

  /**
   * iterates over all orders,
   * skips oracle pegged orders which are invalid due to oracle price crossing the peg limit,
   * skips tif orders which are invalid due to tif having elapsed,
   */
  public *itemsValid(): Generator<PerpOrder> {
    const itemsGen = this.items();
    let itemsRes = itemsGen.next();
    while (true) {
      if (itemsRes.value) {
        const val = itemsRes.value;
        if (
          !val.isExpired &&
          (!val.isOraclePegged ||
            (val.isOraclePegged && !val.oraclePeggedProperties.isInvalid))
        ) {
          yield val;
        }
        itemsRes = itemsGen.next();
      } else {
        break;
      }
    }
  }

  public *fixedItems(): Generator<PerpOrder> {
    if (this.rootFixed.leafCount === 0) {
      return;
    }
    const now = this.now;
    const stack = [this.rootFixed.maybeNode];
    const [left, right] = this.type === BookSideType.bids ? [1, 0] : [0, 1];

    while (stack.length > 0) {
      const index = stack.pop()!;
      const node = this.orderTreeNodes.nodes[index];
      if (node.tag === BookSide.INNER_NODE_TAG) {
        const innerNode = BookSide.toInnerNode(this.client, node.data);
        stack.push(innerNode.children[right], innerNode.children[left]);
      } else if (node.tag === BookSide.LEAF_NODE_TAG) {
        const leafNode = BookSide.toLeafNode(this.client, node.data);
        const expiryTimestamp = leafNode.timeInForce
          ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
          : U64_MAX_BN;

        yield PerpOrder.from(
          this.perpMarket,
          leafNode,
          this.type,
          now.gt(expiryTimestamp),
        );
      }
    }
  }

  public *oraclePeggedItems(): Generator<PerpOrder> {
    if (this.rootOraclePegged.leafCount === 0) {
      return;
    }
    const now = this.now;
    const stack = [this.rootOraclePegged.maybeNode];
    const [left, right] = this.type === BookSideType.bids ? [1, 0] : [0, 1];

    while (stack.length > 0) {
      const index = stack.pop()!;
      const node = this.orderTreeNodes.nodes[index];
      if (node.tag === BookSide.INNER_NODE_TAG) {
        const innerNode = BookSide.toInnerNode(this.client, node.data);
        stack.push(innerNode.children[right], innerNode.children[left]);
      } else if (node.tag === BookSide.LEAF_NODE_TAG) {
        const leafNode = BookSide.toLeafNode(this.client, node.data);
        const expiryTimestamp = leafNode.timeInForce
          ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
          : U64_MAX_BN;

        yield PerpOrder.from(
          this.perpMarket,
          leafNode,
          this.type,
          now.gt(expiryTimestamp),
          true,
        );
      }
    }
  }

  public best(): PerpOrder | undefined {
    return this.items().next().value;
  }

  getImpactPriceUi(baseLots: BN): number | undefined {
    const s = new BN(0);
    for (const order of this.items()) {
      s.iadd(order.sizeLots);
      if (s.gte(baseLots)) {
        return order.uiPrice;
      }
    }
    return undefined;
  }

  public getL2(depth: number): [number, number, BN, BN][] {
    const levels: [BN, BN][] = [];
    for (const { priceLots, sizeLots } of this.items()) {
      if (levels.length > 0 && levels[levels.length - 1][0].eq(priceLots)) {
        levels[levels.length - 1][1].iadd(sizeLots);
      } else if (levels.length === depth) {
        break;
      } else {
        levels.push([priceLots, sizeLots]);
      }
    }
    return levels.map(([priceLots, sizeLots]) => [
      this.perpMarket.priceLotsToUi(priceLots),
      this.perpMarket.baseLotsToUi(sizeLots),
      priceLots,
      sizeLots,
    ]);
  }

  public getL2Ui(depth: number): [number, number][] {
    const levels: [number, number][] = [];
    for (const { uiPrice: price, uiSize: size } of this.items()) {
      if (levels.length > 0 && levels[levels.length - 1][0] === price) {
        levels[levels.length - 1][1] += size;
      } else if (levels.length === depth) {
        break;
      } else {
        levels.push([price, size]);
      }
    }
    return levels;
  }

  static toInnerNode(client: MangoClient, data: [number]): InnerNode {
    return (client.program as any)._coder.types.typeLayouts
      .get('InnerNode')
      .decode(Buffer.from([BookSide.INNER_NODE_TAG].concat(data)));
  }
  static toLeafNode(client: MangoClient, data: [number]): LeafNode {
    return LeafNode.from(
      (client.program as any)._coder.types.typeLayouts
        .get('LeafNode')
        .decode(Buffer.from([BookSide.LEAF_NODE_TAG].concat(data))),
    );
  }
}

export type BookSideType =
  | { bids: Record<string, never> }
  | { asks: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace BookSideType {
  export const bids = { bids: {} };
  export const asks = { asks: {} };
}

export class LeafNode {
  static from(obj: {
    ownerSlot: number;
    orderType: PerpOrderType;
    timeInForce: number;
    key: BN;
    owner: PublicKey;
    quantity: BN;
    timestamp: BN;
    pegLimit: BN;
  }): LeafNode {
    return new LeafNode(
      obj.ownerSlot,
      obj.orderType,
      obj.timeInForce,
      obj.key,
      obj.owner,
      obj.quantity,
      obj.timestamp,
      obj.pegLimit,
    );
  }

  constructor(
    public ownerSlot: number,
    public orderType: PerpOrderType,
    public timeInForce: number,
    public key: BN,
    public owner: PublicKey,
    public quantity: BN,
    public timestamp: BN,
    public pegLimit: BN,
  ) {}
}
export class InnerNode {
  static from(obj: { children: [number] }): InnerNode {
    return new InnerNode(obj.children);
  }

  constructor(public children: [number]) {}
}

export type PerpSelfTradeBehavior =
  | { decrementTake: Record<string, never> }
  | { cancelProvide: Record<string, never> }
  | { abortTransaction: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace PerpSelfTradeBehavior {
  export const decrementTake = { decrementTake: {} };
  export const cancelProvide = { cancelProvide: {} };
  export const abortTransaction = { abortTransaction: {} };
}

export type PerpOrderSide =
  | { bid: Record<string, never> }
  | { ask: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace PerpOrderSide {
  export const bid = { bid: {} };
  export const ask = { ask: {} };
}

export type PerpOrderType =
  | { limit: Record<string, never> }
  | { immediateOrCancel: Record<string, never> }
  | { postOnly: Record<string, never> }
  | { market: Record<string, never> }
  | { postOnlySlide: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace PerpOrderType {
  export const limit = { limit: {} };
  export const immediateOrCancel = { immediateOrCancel: {} };
  export const postOnly = { postOnly: {} };
  export const market = { market: {} };
  export const postOnlySlide = { postOnlySlide: {} };
}

export class PerpOrder {
  static from(
    perpMarket: PerpMarket,
    leafNode: LeafNode,
    type: BookSideType,
    isExpired = false,
    isOraclePegged = false,
  ): PerpOrder {
    const side =
      type == BookSideType.bids ? PerpOrderSide.bid : PerpOrderSide.ask;
    let priceLots;
    let oraclePeggedProperties;
    if (isOraclePegged) {
      const priceData = leafNode.key.ushrn(64);
      const priceOffset = priceData.sub(new BN(1).ushln(63));
      priceLots = perpMarket.uiPriceToLots(perpMarket.uiPrice).add(priceOffset);
      const isInvalid =
        type === BookSideType.bids
          ? priceLots.gt(leafNode.pegLimit) && !leafNode.pegLimit.eqn(-1)
          : leafNode.pegLimit.gt(priceLots);
      oraclePeggedProperties = {
        isInvalid,
        priceOffset,
        uiPriceOffset: perpMarket.priceLotsToUi(priceOffset),
        pegLimit: leafNode.pegLimit,
        uiPegLimit: perpMarket.priceLotsToUi(leafNode.pegLimit),
      } as OraclePeggedProperties;
    } else {
      priceLots = BookSide.getPriceFromKey(leafNode.key);
    }
    const expiryTimestamp = leafNode.timeInForce
      ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
      : U64_MAX_BN;

    return new PerpOrder(
      type === BookSideType.bids
        ? RUST_U64_MAX().sub(leafNode.key.maskn(64))
        : leafNode.key.maskn(64),
      leafNode.key,
      leafNode.owner,
      leafNode.ownerSlot,
      0,
      perpMarket.priceLotsToUi(priceLots),
      priceLots,
      perpMarket.baseLotsToUi(leafNode.quantity),
      leafNode.quantity,
      side,
      leafNode.timestamp,
      expiryTimestamp,
      perpMarket.perpMarketIndex,
      isExpired,
      isOraclePegged,
      leafNode.orderType,
      oraclePeggedProperties,
    );
  }

  constructor(
    public seqNum: BN,
    public orderId: BN,
    public owner: PublicKey,
    public openOrdersSlot: number,
    public feeTier: 0,
    public uiPrice: number,
    public priceLots: BN,
    public uiSize: number,
    public sizeLots: BN,
    public side: PerpOrderSide,
    public timestamp: BN,
    public expiryTimestamp: BN,
    public perpMarketIndex: number,
    public isExpired = false,
    public isOraclePegged = false,
    public orderType: PerpOrderType,
    public oraclePeggedProperties?: OraclePeggedProperties,
  ) {}

  get price(): number {
    return this.uiPrice;
  }

  get size(): number {
    return this.uiSize;
  }
}

interface OraclePeggedProperties {
  isInvalid: boolean;
  priceOffset: BN;
  uiPriceOffset: number;
  pegLimit: BN;
  uiPegLimit: number;
}

export class PerpEventQueue {
  static FILL_EVENT_TYPE = 0;
  static OUT_EVENT_TYPE = 1;
  static LIQUIDATE_EVENT_TYPE = 2;
  public head: number;
  public count: number;
  public seqNum: BN;
  public rawEvents: (OutEvent | FillEvent | LiquidateEvent)[];
  constructor(
    client: MangoClient,
    header: { head: number; count: number; seqNum: BN },
    buf,
  ) {
    this.head = header.head;
    this.count = header.count;
    this.seqNum = header.seqNum;
    this.rawEvents = buf.map((event) => {
      if (event.eventType === PerpEventQueue.FILL_EVENT_TYPE) {
        return (client.program as any)._coder.types.typeLayouts
          .get('FillEvent')
          .decode(
            Buffer.from([PerpEventQueue.FILL_EVENT_TYPE].concat(event.padding)),
          );
      } else if (event.eventType === PerpEventQueue.OUT_EVENT_TYPE) {
        return (client.program as any)._coder.types.typeLayouts
          .get('OutEvent')
          .decode(
            Buffer.from([PerpEventQueue.OUT_EVENT_TYPE].concat(event.padding)),
          );
      } else if (event.eventType === PerpEventQueue.LIQUIDATE_EVENT_TYPE) {
        return (client.program as any)._coder.types.typeLayouts
          .get('LiquidateEvent')
          .decode(
            Buffer.from(
              [PerpEventQueue.LIQUIDATE_EVENT_TYPE].concat(event.padding),
            ),
          );
      }
      throw new Error(`Unknown event with eventType ${event.eventType}!`);
    });
  }

  public getUnconsumedEvents(): (OutEvent | FillEvent | LiquidateEvent)[] {
    const events: (OutEvent | FillEvent | LiquidateEvent)[] = [];
    const head = this.head;
    for (let i = 0; i < this.count; i++) {
      events.push(this.rawEvents[(head + i) % this.rawEvents.length]);
    }
    return events;
  }

  public eventsSince(
    lastSeqNum?: BN,
  ): (OutEvent | FillEvent | LiquidateEvent)[] {
    return this.rawEvents
      .filter((e) =>
        e.seqNum.gt(lastSeqNum === undefined ? new BN(0) : lastSeqNum),
      )
      .sort((a, b) => a.seqNum.cmp(b.seqNum));
  }
}

export interface Event {
  eventType: number;
}

export interface OutEvent extends Event {
  side: PerpOrderType;
  ownerSlot: number;
  timestamp: BN;
  seqNum: BN;
  owner: PublicKey;
  quantity: BN;
}

export interface FillEvent extends Event {
  takerSide: 0 | 1; // 0 = bid, 1 = ask
  makerOut: boolean;
  makerSlot: number;
  timestamp: BN;
  seqNum: BN;
  maker: PublicKey;
  makerOrderId: BN;
  makerFee: number;
  makerTimestamp: BN;
  taker: PublicKey;
  takerOrderId: BN;
  takerClientOrderId: BN;
  takerFee: number;
  price: number;
  quantity: number;
}

export interface LiquidateEvent extends Event {
  seqNum: BN;
}
