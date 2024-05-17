import BN from "bn.js";
import { MangoClient, PerpMarket, RUST_U64_MAX, U64_MAX_BN } from "..";
import { PublicKey } from "@solana/web3.js";

interface BookSideAccount {
  roots: OrderTreeRoot[];
  nodes: OrderTreeNodes;
}

interface OrderTreeNodes {
  bumpIndex: number;
  freeListLen: number;
  freeListHead: number;
  nodes: AnyNode[]
}

interface AnyNode {
  tag: number; data?: number[], nodeData?: Buffer;
}

interface OrderTreeRoot {
  maybeNode: number;
  leafCount: number;
}

function decodeOrderTreeRootStruct(data: Buffer): OrderTreeRoot {
  const maybeNode = data.readUInt32LE(0);
  const leafCount = data.readUInt32LE(4);
  return { maybeNode, leafCount };
}


export class BookSide {
  private static INNER_NODE_TAG = 1;
  private static LEAF_NODE_TAG = 2;
  now: BN;

  static from(
    client: MangoClient,
    perpMarket: PerpMarket,
    bookSideType: BookSideType,
    account: BookSideAccount,
  ): BookSide {
    return new BookSide(
      client,
      perpMarket,
      bookSideType,
      account
    );
  }

  static decodeAccountfromBuffer(
    data: Buffer
  ): BookSideAccount {
    // TODO: add discriminator parsing & check
    const roots = [
      decodeOrderTreeRootStruct(data.subarray(8)),
      decodeOrderTreeRootStruct(data.subarray(16)),
    ];

    // skip reserved
    let offset = 56 + 256;

    const orderTreeType = data.readUInt8(offset);
    const bumpIndex = data.readUInt32LE(offset + 4);
    const freeListLen = data.readUInt32LE(offset + 8);
    const freeListHead = data.readUInt32LE(offset + 12);

    // skip more reserved data
    offset += 16 + 512;

    const nodes: { tag: number, nodeData: Buffer }[] = [];
    for (let i = 0; i < 1024; ++i) {
      const tag = data.readUInt8(offset);
      const nodeData = data.subarray(offset, offset + 88);
      nodes.push({ tag, nodeData });
      offset += 88;
    }

    // this result has a slightly different layout than the regular account
    // it doesn't include reserved data and it's AnyNodes don't have the field
    // data: number[] (excluding the tag prefix byte)
    // but nodeData: Buffer (including the tag prefix byte)
    const result = {
      roots,
      nodes: { orderTreeType, bumpIndex, freeListLen, freeListHead, nodes },
    };

    return result;
  }

  constructor(
    public client: MangoClient,
    public perpMarket: PerpMarket,
    public type: BookSideType,
    public account: BookSideAccount,
    maxBookDelay?: number,
  ) {
    // Determine the maxTimestamp found on the book to use for tif
    // If maxBookDelay is not provided, use 3600 as a very large number
    maxBookDelay = maxBookDelay === undefined ? 3600 : maxBookDelay;
    let maxTimestamp = new BN(new Date().getTime() / 1000 - maxBookDelay);

    for (const node of account.nodes.nodes) {
      if (node.tag !== BookSide.LEAF_NODE_TAG) {
        continue;
      }

      const leafNode = BookSide.toLeafNode(client, node);
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
      const node = this.account.nodes.nodes[index];
      if (node.tag === BookSide.INNER_NODE_TAG) {
        const innerNode = BookSide.toInnerNode(this.client, node);
        stack.push(innerNode.children[right], innerNode.children[left]);
      } else if (node.tag === BookSide.LEAF_NODE_TAG) {
        const leafNode = BookSide.toLeafNode(this.client, node);
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
      const node = this.account.nodes.nodes[index];
      if (node.tag === BookSide.INNER_NODE_TAG) {
        const innerNode = BookSide.toInnerNode(this.client, node);
        stack.push(innerNode.children[right], innerNode.children[left]);
      } else if (node.tag === BookSide.LEAF_NODE_TAG) {
        const leafNode = BookSide.toLeafNode(this.client, node);
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

  get rootFixed(): OrderTreeRoot { return this.account.roots[0]; }
  get rootOraclePegged(): OrderTreeRoot { return this.account.roots[1]; }

  static toInnerNode(client: MangoClient, node: AnyNode): InnerNode {
    const layout = (client.program as any)._coder.types.typeLayouts
      .get('InnerNode');
    if (node.nodeData) {
      return layout.decode(node.nodeData);
    }
    return layout
      .decode(Buffer.from([BookSide.INNER_NODE_TAG].concat(node.data!)));
  }

  static toLeafNode(client: MangoClient, node: AnyNode): LeafNode {
    const layout = (client.program as any)._coder.types.typeLayouts
      .get('LeafNode');
    if (node.nodeData) {
      return layout.decode(node.nodeData);
  }
    return layout
      .decode(Buffer.from([BookSide.LEAF_NODE_TAG].concat(node.data!)));
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
  ) { }
}
export class InnerNode {
  static from(obj: { children: [number] }): InnerNode {
    return new InnerNode(obj.children);
  }

  constructor(public children: [number]) { }
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
  ) { }

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