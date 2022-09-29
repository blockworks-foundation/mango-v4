import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import Big from 'big.js';
import { MangoClient } from '../client';
import { U64_MAX_BN } from '../utils';
import { OracleConfig, QUOTE_DECIMALS } from './bank';
import { I80F48, I80F48Dto } from './I80F48';

export class PerpMarket {
  public name: string;
  public maintAssetWeight: I80F48;
  public initAssetWeight: I80F48;
  public maintLiabWeight: I80F48;
  public initLiabWeight: I80F48;
  public liquidationFee: I80F48;
  public makerFee: I80F48;
  public takerFee: I80F48;
  public minFunding: I80F48;
  public maxFunding: I80F48;
  public longFunding: I80F48;
  public shortFunding: I80F48;
  public openInterest: number;
  public seqNum: number;
  public feesAccrued: I80F48;
  priceLotsToUiConverter: number;
  baseLotsToUiConverter: number;
  quoteLotsToUiConverter: number;
  public _price: I80F48;
  public _uiPrice: number;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      quoteTokenIndex: number;
      perpMarketIndex: number;
      name: number[];
      oracle: PublicKey;
      oracleConfig: OracleConfig;
      bids: PublicKey;
      asks: PublicKey;
      eventQueue: PublicKey;
      quoteLotSize: BN;
      baseLotSize: BN;
      maintAssetWeight: I80F48Dto;
      initAssetWeight: I80F48Dto;
      maintLiabWeight: I80F48Dto;
      initLiabWeight: I80F48Dto;
      liquidationFee: I80F48Dto;
      makerFee: I80F48Dto;
      takerFee: I80F48Dto;
      minFunding: I80F48Dto;
      maxFunding: I80F48Dto;
      impactQuantity: BN;
      longFunding: I80F48Dto;
      shortFunding: I80F48Dto;
      fundingLastUpdated: BN;
      openInterest: BN;
      seqNum: any; // TODO: ts complains that this is unknown for whatever reason
      feesAccrued: I80F48Dto;
      bump: number;
      baseDecimals: number;
      registrationTime: BN;
    },
  ): PerpMarket {
    return new PerpMarket(
      publicKey,
      obj.group,
      obj.quoteTokenIndex,
      obj.perpMarketIndex,
      obj.name,
      obj.oracle,
      obj.oracleConfig,
      obj.bids,
      obj.asks,
      obj.eventQueue,
      obj.quoteLotSize,
      obj.baseLotSize,
      obj.maintAssetWeight,
      obj.initAssetWeight,
      obj.maintLiabWeight,
      obj.initLiabWeight,
      obj.liquidationFee,
      obj.makerFee,
      obj.takerFee,
      obj.minFunding,
      obj.maxFunding,
      obj.impactQuantity,
      obj.longFunding,
      obj.shortFunding,
      obj.fundingLastUpdated,
      obj.openInterest,
      obj.seqNum,
      obj.feesAccrued,
      obj.bump,
      obj.baseDecimals,
      obj.registrationTime,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public quoteTokenIndex: number,
    public perpMarketIndex: number, // TODO rename to marketIndex?
    name: number[],
    public oracle: PublicKey,
    oracleConfig: OracleConfig,
    public bids: PublicKey,
    public asks: PublicKey,
    public eventQueue: PublicKey,
    public quoteLotSize: BN,
    public baseLotSize: BN,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    liquidationFee: I80F48Dto,
    makerFee: I80F48Dto,
    takerFee: I80F48Dto,
    minFunding: I80F48Dto,
    maxFunding: I80F48Dto,
    public impactQuantity: BN,
    longFunding: I80F48Dto,
    shortFunding: I80F48Dto,
    fundingLastUpdated: BN,
    openInterest: BN,
    seqNum: BN,
    feesAccrued: I80F48Dto,
    bump: number,
    public baseDecimals: number,
    public registrationTime: BN,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.maintAssetWeight = I80F48.from(maintAssetWeight);
    this.initAssetWeight = I80F48.from(initAssetWeight);
    this.maintLiabWeight = I80F48.from(maintLiabWeight);
    this.initLiabWeight = I80F48.from(initLiabWeight);
    this.liquidationFee = I80F48.from(liquidationFee);
    this.makerFee = I80F48.from(makerFee);
    this.takerFee = I80F48.from(takerFee);
    this.minFunding = I80F48.from(minFunding);
    this.maxFunding = I80F48.from(maxFunding);
    this.longFunding = I80F48.from(longFunding);
    this.shortFunding = I80F48.from(shortFunding);
    this.openInterest = openInterest.toNumber();
    this.seqNum = seqNum.toNumber();
    this.feesAccrued = I80F48.from(feesAccrued);

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

  get price(): I80F48 {
    if (!this._price) {
      throw new Error(
        `Undefined price for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`,
      );
    }
    return this._price;
  }

  get uiPrice(): number {
    if (!this._uiPrice) {
      throw new Error(
        `Undefined price for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`,
      );
    }
    return this._uiPrice;
  }
  public async loadAsks(client: MangoClient): Promise<BookSide> {
    const asks = await client.program.account.bookSide.fetch(this.asks);
    return BookSide.from(client, this, BookSideType.asks, asks);
  }

  public async loadBids(client: MangoClient): Promise<BookSide> {
    const bids = await client.program.account.bookSide.fetch(this.bids);
    return BookSide.from(client, this, BookSideType.bids, bids);
  }

  public async loadEventQueue(client: MangoClient): Promise<PerpEventQueue> {
    const eventQueue = await client.program.account.eventQueue.fetch(
      this.eventQueue,
    );
    return new PerpEventQueue(client, eventQueue.header, eventQueue.buf);
  }

  public async loadFills(
    client: MangoClient,
    lastSeqNum: BN,
  ): Promise<(OutEvent | FillEvent | LiquidateEvent)[]> {
    const eventQueue = await this.loadEventQueue(client);
    return eventQueue
      .eventsSince(lastSeqNum)
      .filter((event) => event.eventType == PerpEventQueue.FILL_EVENT_TYPE);
  }

  /**
   *
   * @param bids
   * @param asks
   * @returns returns funding rate per hour
   */
  public getCurrentFundingRate(bids: BookSide, asks: BookSide): number {
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
    return funding / 24;
  }

  public uiPriceToLots(price: number): BN {
    return new BN(price * Math.pow(10, QUOTE_DECIMALS))
      .mul(this.baseLotSize)
      .div(this.quoteLotSize.mul(new BN(Math.pow(10, this.baseDecimals))));
  }

  public uiBaseToLots(quantity: number): BN {
    return new BN(quantity * Math.pow(10, this.baseDecimals)).div(
      this.baseLotSize,
    );
  }

  public uiQuoteToLots(uiQuote: number): BN {
    return new BN(uiQuote * Math.pow(10, QUOTE_DECIMALS)).div(
      this.quoteLotSize,
    );
  }

  public priceLotsToUi(price: BN): number {
    return parseFloat(price.toString()) * this.priceLotsToUiConverter;
  }

  public baseLotsToUi(quantity: BN): number {
    return parseFloat(quantity.toString()) * this.baseLotsToUiConverter;
  }

  public quoteLotsToUi(quantity: BN): number {
    return parseFloat(quantity.toString()) * this.quoteLotsToUiConverter;
  }

  toString(): string {
    return (
      'PerpMarket ' +
      '\n perpMarketIndex -' +
      this.perpMarketIndex +
      '\n maintAssetWeight -' +
      this.maintAssetWeight.toNumber() +
      '\n initAssetWeight -' +
      this.initAssetWeight.toNumber() +
      '\n maintLiabWeight -' +
      this.maintLiabWeight.toNumber() +
      '\n initLiabWeight -' +
      this.initLiabWeight.toNumber() +
      '\n liquidationFee -' +
      this.liquidationFee.toNumber() +
      '\n makerFee -' +
      this.makerFee.toNumber() +
      '\n takerFee -' +
      this.takerFee.toNumber()
    );
  }
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
      bumpIndex: number;
      freeListLen: number;
      freeListHead: number;
      rootNode: number;
      leafCount: number;
      nodes: unknown;
    },
  ): BookSide {
    return new BookSide(
      client,
      perpMarket,
      bookSideType,
      obj.bumpIndex,
      obj.freeListLen,
      obj.freeListHead,
      obj.rootNode,
      obj.leafCount,
      obj.nodes,
    );
  }

  constructor(
    public client: MangoClient,
    public perpMarket: PerpMarket,
    public type: BookSideType,
    public bumpIndex,
    public freeListLen,
    public freeListHead,
    public rootNode,
    public leafCount,
    public nodes,
    public includeExpired = false,
    maxBookDelay?: number,
  ) {
    // TODO why? Ask Daffy
    // Determine the maxTimestamp found on the book to use for tif
    // If maxBookDelay is not provided, use 3600 as a very large number
    maxBookDelay = maxBookDelay === undefined ? 3600 : maxBookDelay;
    let maxTimestamp = new BN(new Date().getTime() / 1000 - maxBookDelay);
    for (const node of this.nodes) {
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

  public *items(): Generator<PerpOrder> {
    if (this.leafCount === 0) {
      return;
    }
    const now = this.now;
    const stack = [this.rootNode];
    const [left, right] = this.type === BookSideType.bids ? [1, 0] : [0, 1];

    while (stack.length > 0) {
      const index = stack.pop();
      const node = this.nodes[index];
      if (node.tag === BookSide.INNER_NODE_TAG) {
        const innerNode = BookSide.toInnerNode(this.client, node.data);
        stack.push(innerNode.children[right], innerNode.children[left]);
      } else if (node.tag === BookSide.LEAF_NODE_TAG) {
        const leafNode = BookSide.toLeafNode(this.client, node.data);
        const expiryTimestamp = leafNode.timeInForce
          ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
          : U64_MAX_BN;
        if (now.lt(expiryTimestamp) || this.includeExpired) {
          yield PerpOrder.from(this.perpMarket, leafNode, this.type);
        }
      }
    }
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
      .decode(Buffer.from([BookSide.INNER_NODE_TAG, 0, 0, 0].concat(data)));
  }
  static toLeafNode(client: MangoClient, data: [number]): LeafNode {
    return LeafNode.from(
      (client.program as any)._coder.types.typeLayouts
        .get('LeafNode')
        .decode(Buffer.from([BookSide.LEAF_NODE_TAG, 0, 0, 0].concat(data))),
    );
  }
}

export class BookSideType {
  static bids = { bids: {} };
  static asks = { asks: {} };
}
export class LeafNode {
  static from(obj: {
    ownerSlot: number;
    orderType: PerpOrderType;
    timeInForce: number;
    key: BN;
    owner: PublicKey;
    quantity: BN;
    clientOrderId: BN;
    timestamp: BN;
  }): LeafNode {
    return new LeafNode(
      obj.ownerSlot,
      obj.orderType,
      obj.timeInForce,
      obj.key,
      obj.owner,
      obj.quantity,
      obj.clientOrderId,
      obj.timestamp,
    );
  }

  constructor(
    public ownerSlot: number,
    public orderType: PerpOrderType,
    public timeInForce: number,
    public key: BN,
    public owner: PublicKey,
    public quantity: BN,
    public clientOrderId: BN,
    public timestamp: BN,
  ) {}
}
export class InnerNode {
  static from(obj: { children: [number] }): InnerNode {
    return new InnerNode(obj.children);
  }

  constructor(public children: [number]) {}
}

export class PerpOrderSide {
  static bid = { bid: {} };
  static ask = { ask: {} };
}

export class PerpOrderType {
  static limit = { limit: {} };
  static immediateOrCancel = { immediateorcancel: {} };
  static postOnly = { postonly: {} };
  static market = { market: {} };
  static postOnlySlide = { postonlyslide: {} };
}

export class PerpOrder {
  static from(
    perpMarket: PerpMarket,
    leafNode: LeafNode,
    type: BookSideType,
  ): PerpOrder {
    const side =
      type == BookSideType.bids ? PerpOrderSide.bid : PerpOrderSide.ask;
    const price = BookSide.getPriceFromKey(leafNode.key);
    const expiryTimestamp = leafNode.timeInForce
      ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
      : U64_MAX_BN;

    return new PerpOrder(
      leafNode.key,
      leafNode.clientOrderId,
      leafNode.owner,
      leafNode.ownerSlot,
      0,
      perpMarket.priceLotsToUi(price),
      price,
      perpMarket.baseLotsToUi(leafNode.quantity),
      leafNode.quantity,
      side,
      leafNode.timestamp,
      expiryTimestamp,
    );
  }

  constructor(
    public orderId: BN,
    public clientId: BN,
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
  ) {}
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
  takerSide: PerpOrderType;
  makerOut: boolean;
  makerSlot: number;
  marketFeesApplied: boolean;
  timestamp: BN;
  seqNum: BN;
  maker: PublicKey;
  makerOrderId: BN;
  makerClientOrderId: BN;
  makerFee: I80F48;
  makerTimestamp: BN;
  taker: PublicKey;
  takerOrderId: BN;
  takerClientOrderId: BN;
  takerFee: I80F48;
  price: BN;
  quantity: BN;
}

export interface LiquidateEvent extends Event {
  seqNum: BN;
}
