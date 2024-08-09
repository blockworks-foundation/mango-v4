import { BN } from '@coral-xyz/anchor';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import Big from 'big.js';
import { MangoClient } from '../client';
import { I80F48, I80F48Dto, ZERO_I80F48 } from '../numbers/I80F48';
import { Modify } from '../types';
import { As, QUOTE_DECIMALS, toNative, toUiDecimals } from '../utils';
import {
  OracleConfig,
  OracleConfigDto,
  StablePriceModel,
  TokenIndex,
} from './bank';
import { Group } from './group';
import { MangoAccount } from './mangoAccount';
import { OracleProvider, isOracleStaleOrUnconfident } from './oracle';
import {
  BookSide,
  BookSideType,
  PerpOrderSide,
  PerpOrderType,
} from './bookSide';

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
      const askInfo = await client.connection.getAccountInfo(this.asks);
      const asks = BookSide.decodeAccountfromBuffer(askInfo!.data);
      this._asks = BookSide.from(client, this, BookSideType.asks, asks as any);
    }
    return this._asks;
  }

  public async loadBids(
    client: MangoClient,
    forceReload = false,
  ): Promise<BookSide> {
    if (forceReload || !this._bids) {
      const bidInfo = await client.connection.getAccountInfo(this.bids);
      const bids = BookSide.decodeAccountfromBuffer(bidInfo!.data);
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
