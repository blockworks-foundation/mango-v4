import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import Big from 'big.js';
import { MangoClient } from '../client';
import { I80F48, I80F48Dto } from '../numbers/I80F48';
import { As, toNative, U64_MAX_BN } from '../utils';
import { OracleConfig, QUOTE_DECIMALS, TokenIndex } from './bank';

export type PerpMarketIndex = number & As<'perp-market-index'>;

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
  public feesAccrued: I80F48;
  public feesSettled: I80F48;
  public _price: I80F48;
  public _uiPrice: number;

  private priceLotsToUiConverter: number;
  private baseLotsToUiConverter: number;
  private quoteLotsToUiConverter: number;

  private _orderbook: Orderbook;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      settleTokenIndex: number;
      perpMarketIndex: number;
      trustedMarket: number;
      groupInsuranceFund: number;
      name: number[];
      oracle: PublicKey;
      oracleConfig: OracleConfig;
      orderbook: PublicKey;
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
      seqNum: BN;
      feesAccrued: I80F48Dto;
      baseDecimals: number;
      registrationTime: BN;
      feesSettled: I80F48Dto;
      feePenalty: number;
      settleFeeFlat: number;
      settleFeeAmountThreshold: number;
      settleFeeFractionLowHealth: number;
    },
  ): PerpMarket {
    return new PerpMarket(
      publicKey,
      obj.group,
      obj.settleTokenIndex as TokenIndex,
      obj.perpMarketIndex as PerpMarketIndex,
      obj.trustedMarket == 1,
      obj.groupInsuranceFund == 1,
      obj.name,
      obj.oracle,
      obj.oracleConfig,
      obj.orderbook,
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
      obj.baseDecimals,
      obj.registrationTime,
      obj.feesSettled,
      obj.feePenalty,
      obj.settleFeeFlat,
      obj.settleFeeAmountThreshold,
      obj.settleFeeFractionLowHealth,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public settleTokenIndex: TokenIndex,
    public perpMarketIndex: PerpMarketIndex, // TODO rename to marketIndex?
    public trustedMarket: boolean,
    public groupInsuranceFund: boolean,
    name: number[],
    public oracle: PublicKey,
    oracleConfig: OracleConfig,
    public orderbook: PublicKey,
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
    public fundingLastUpdated: BN,
    public openInterest: BN,
    public seqNum: BN,
    feesAccrued: I80F48Dto,
    public baseDecimals: number,
    public registrationTime: BN,
    feesSettled: I80F48Dto,
    public feePenalty: number,
    public settleFeeFlat: number,
    public settleFeeAmountThreshold: number,
    public settleFeeFractionLowHealth: number,
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
    this.feesAccrued = I80F48.from(feesAccrued);
    this.feesSettled = I80F48.from(feesSettled);

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

  get minOrderSize(): number {
    return this.baseLotsToUiConverter;
  }

  get tickSize(): number {
    return this.priceLotsToUiConverter;
  }

  private async loadOrderbook(
    client: MangoClient,
    forceReload = false,
  ): Promise<void> {
    if (forceReload || !this._orderbook)
      this._orderbook = await client.program.account.orderbook.fetch(
        this.orderbook,
      );
  }

  public async loadAsks(client: MangoClient): Promise<BookSide> {
    await this.loadOrderbook(client);
    return BookSide.from(
      client,
      this,
      BookSideType.asks,
      (this._orderbook as Orderbook).asks,
    );
  }

  public async loadBids(client: MangoClient): Promise<BookSide> {
    await this.loadOrderbook(client);
    return BookSide.from(
      client,
      this,
      BookSideType.bids,
      (this._orderbook as Orderbook).bids,
    );
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

  public async logOb(client: MangoClient): Promise<string> {
    let res = ``;
    res += `  ${this.name} OrderBook`;
    let orders = await this?.loadAsks(client);
    for (const order of orders!.items()) {
      res += `\n  ${order.clientId.toString()}  ${order.uiPrice
        .toFixed(5)
        .padStart(10)}, ${order.uiSize.toString().padStart(10)} ${
        order.isOraclePegged && order.oraclePeggedProperties
          ? order.oraclePeggedProperties.pegLimit.toNumber() + ' (PegLimit)'
          : ''
      }`;
    }
    res += `\n  asks ↑ --------- ↓ bids`;
    orders = await this?.loadBids(client);
    for (const order of orders!.items()) {
      res += `\n  ${order.clientId.toString()} ${order.uiPrice
        .toFixed(5)
        .padStart(10)}, ${order.uiSize.toString().padStart(10)} ${
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
      this.maintAssetWeight.toString() +
      '\n initAssetWeight -' +
      this.initAssetWeight.toString() +
      '\n maintLiabWeight -' +
      this.maintLiabWeight.toString() +
      '\n initLiabWeight -' +
      this.initLiabWeight.toString() +
      '\n liquidationFee -' +
      this.liquidationFee.toString() +
      '\n makerFee -' +
      this.makerFee.toString() +
      '\n takerFee -' +
      this.takerFee.toString()
    );
  }
}

interface Orderbook {
  bids: OrderTree;
  asks: OrderTree;
}

interface OrderTree {
  fixed: {
    bumpIndex: number;
    freeListLen: number;
    freeListHead: number;
    rootNode: number;
    leafCount: number;
    nodes: [any];
  };
  oraclePegged: {
    bumpIndex: number;
    freeListLen: number;
    freeListHead: number;
    rootNode: number;
    leafCount: number;
    nodes: [any];
  };
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
      fixed: {
        bumpIndex: number;
        freeListLen: number;
        freeListHead: number;
        rootNode: number;
        leafCount: number;
        nodes: [any];
      };
      oraclePegged: {
        bumpIndex: number;
        freeListLen: number;
        freeListHead: number;
        rootNode: number;
        leafCount: number;
        nodes: [any];
      };
    },
  ): BookSide {
    return new BookSide(client, perpMarket, bookSideType, obj);
  }

  constructor(
    public client: MangoClient,
    public perpMarket: PerpMarket,
    public type: BookSideType,
    public orderTree: {
      fixed: {
        bumpIndex: number;
        freeListLen: number;
        freeListHead: number;
        rootNode: number;
        leafCount: number;
        nodes: [any];
      };
      oraclePegged: {
        bumpIndex: number;
        freeListLen: number;
        freeListHead: number;
        rootNode: number;
        leafCount: number;
        nodes: [any];
      };
    },
    maxBookDelay?: number,
  ) {
    // Determine the maxTimestamp found on the book to use for tif
    // If maxBookDelay is not provided, use 3600 as a very large number
    maxBookDelay = maxBookDelay === undefined ? 3600 : maxBookDelay;
    let maxTimestamp = new BN(new Date().getTime() / 1000 - maxBookDelay);
    for (const node of [
      ...this.orderTree.fixed.nodes,
      ...this.orderTree.oraclePegged.nodes,
    ]) {
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
    function isBetter(type, a, b): boolean {
      return type === BookSideType.bids ? a > b : b < a;
    }

    const fGen = this.fixedItems();
    const oPegGen = this.oraclePeggedItems();

    let fOrderRes = fGen.next();
    let oPegOrderRes = oPegGen.next();

    while (true) {
      if (fOrderRes.value && oPegOrderRes.value) {
        if (isBetter(this.type, fOrderRes.value.key, oPegOrderRes.value.key)) {
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
          !val.isExpired ||
          !val.isOraclePegged ||
          (val.isOraclePegged && !val.oraclePeggedProperties.isInvalid)
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
    if (this.orderTree.fixed.leafCount === 0) {
      return;
    }
    const now = this.now;
    const stack = [this.orderTree.fixed.rootNode];
    const [left, right] = this.type === BookSideType.bids ? [1, 0] : [0, 1];

    while (stack.length > 0) {
      const index = stack.pop()!;
      const node = this.orderTree.fixed.nodes[index];
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
          now.lt(expiryTimestamp),
        );
      }
    }
  }

  public *oraclePeggedItems(): Generator<PerpOrder> {
    if (this.orderTree.oraclePegged.leafCount === 0) {
      return;
    }
    const now = this.now;
    const stack = [this.orderTree.oraclePegged.rootNode];
    const [left, right] = this.type === BookSideType.bids ? [1, 0] : [0, 1];

    while (stack.length > 0) {
      const index = stack.pop()!;
      const node = this.orderTree.oraclePegged.nodes[index];
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
          now.lt(expiryTimestamp),
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
    pegLimit: BN;
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
    public clientOrderId: BN,
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

export class PerpOrderSide {
  static bid = { bid: {} };
  static ask = { ask: {} };
}

export class PerpOrderType {
  static limit = { limit: {} };
  static immediateOrCancel = { immediateOrCancel: {} };
  static postOnly = { postOnly: {} };
  static market = { market: {} };
  static postOnlySlide = { postOnlySlide: {} };
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
    let price;
    let oraclePeggedProperties;
    if (isOraclePegged) {
      const priceData = leafNode.key.ushrn(64);
      const priceOffset = priceData.sub(new BN(1).ushln(63));
      price = perpMarket.uiPriceToLots(perpMarket.uiPrice).add(priceOffset);
      const isInvalid =
        type === BookSideType.bids
          ? price.gt(leafNode.pegLimit)
          : leafNode.pegLimit.gt(price);
      oraclePeggedProperties = {
        isInvalid,
        priceOffset,
        uiPriceOffset: perpMarket.priceLotsToUi(priceOffset),
        pegLimit: leafNode.pegLimit,
        uiPegLimit: perpMarket.priceLotsToUi(leafNode.pegLimit),
      } as OraclePeggedProperties;
    } else {
      price = BookSide.getPriceFromKey(leafNode.key);
    }
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
      perpMarket.perpMarketIndex,
      isOraclePegged,
      oraclePeggedProperties,
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
    public perpMarketIndex: number,
    public isOraclePegged = false,
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
