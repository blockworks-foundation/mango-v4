import { BN } from '@coral-xyz/anchor';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import Big from 'big.js';
import { RUST_U64_MAX } from '../constants';
import { I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { QUOTE_DECIMALS, U64_MAX_BN, toNative, toUiDecimals, } from '../utils';
export class PerpMarket {
    publicKey;
    group;
    settleTokenIndex;
    perpMarketIndex;
    groupInsuranceFund;
    baseDecimals;
    bids;
    asks;
    eventQueue;
    oracle;
    stablePriceModel;
    quoteLotSize;
    baseLotSize;
    openInterest;
    seqNum;
    registrationTime;
    impactQuantity;
    fundingLastUpdated;
    feePenalty;
    settleFeeFlat;
    settleFeeAmountThreshold;
    settleFeeFractionLowHealth;
    settlePnlLimitFactor;
    settlePnlLimitWindowSizeTs;
    reduceOnly;
    name;
    oracleConfig;
    maintBaseAssetWeight;
    initBaseAssetWeight;
    maintBaseLiabWeight;
    initBaseLiabWeight;
    baseLiquidationFee;
    makerFee;
    takerFee;
    minFunding;
    maxFunding;
    longFunding;
    shortFunding;
    feesAccrued;
    feesSettled;
    maintOverallAssetWeight;
    initOverallAssetWeight;
    positivePnlLiquidationFee;
    _price;
    _uiPrice;
    _oracleLastUpdatedSlot;
    _bids;
    _asks;
    priceLotsToUiConverter;
    baseLotsToUiConverter;
    quoteLotsToUiConverter;
    static from(publicKey, obj) {
        return new PerpMarket(publicKey, obj.group, obj.settleTokenIndex, obj.perpMarketIndex, obj.groupInsuranceFund == 1, obj.baseDecimals, obj.name, obj.bids, obj.asks, obj.eventQueue, obj.oracle, obj.oracleConfig, obj.stablePriceModel, obj.quoteLotSize, obj.baseLotSize, obj.maintBaseAssetWeight, obj.initBaseAssetWeight, obj.maintBaseLiabWeight, obj.initBaseLiabWeight, obj.openInterest, obj.seqNum, obj.registrationTime, obj.minFunding, obj.maxFunding, obj.impactQuantity, obj.longFunding, obj.shortFunding, obj.fundingLastUpdated, obj.baseLiquidationFee, obj.makerFee, obj.takerFee, obj.feesAccrued, obj.feesSettled, obj.feePenalty, obj.settleFeeFlat, obj.settleFeeAmountThreshold, obj.settleFeeFractionLowHealth, obj.settlePnlLimitFactor, obj.settlePnlLimitWindowSizeTs, obj.reduceOnly == 1, obj.maintOverallAssetWeight, obj.initOverallAssetWeight, obj.positivePnlLiquidationFee);
    }
    constructor(publicKey, group, settleTokenIndex, perpMarketIndex, // TODO rename to marketIndex?
    groupInsuranceFund, baseDecimals, name, bids, asks, eventQueue, oracle, oracleConfig, stablePriceModel, quoteLotSize, baseLotSize, maintBaseAssetWeight, initBaseAssetWeight, maintBaseLiabWeight, initBaseLiabWeight, openInterest, seqNum, registrationTime, minFunding, maxFunding, impactQuantity, longFunding, shortFunding, fundingLastUpdated, baseLiquidationFee, makerFee, takerFee, feesAccrued, feesSettled, feePenalty, settleFeeFlat, settleFeeAmountThreshold, settleFeeFractionLowHealth, settlePnlLimitFactor, settlePnlLimitWindowSizeTs, reduceOnly, maintOverallAssetWeight, initOverallAssetWeight, positivePnlLiquidationFee) {
        this.publicKey = publicKey;
        this.group = group;
        this.settleTokenIndex = settleTokenIndex;
        this.perpMarketIndex = perpMarketIndex;
        this.groupInsuranceFund = groupInsuranceFund;
        this.baseDecimals = baseDecimals;
        this.bids = bids;
        this.asks = asks;
        this.eventQueue = eventQueue;
        this.oracle = oracle;
        this.stablePriceModel = stablePriceModel;
        this.quoteLotSize = quoteLotSize;
        this.baseLotSize = baseLotSize;
        this.openInterest = openInterest;
        this.seqNum = seqNum;
        this.registrationTime = registrationTime;
        this.impactQuantity = impactQuantity;
        this.fundingLastUpdated = fundingLastUpdated;
        this.feePenalty = feePenalty;
        this.settleFeeFlat = settleFeeFlat;
        this.settleFeeAmountThreshold = settleFeeAmountThreshold;
        this.settleFeeFractionLowHealth = settleFeeFractionLowHealth;
        this.settlePnlLimitFactor = settlePnlLimitFactor;
        this.settlePnlLimitWindowSizeTs = settlePnlLimitWindowSizeTs;
        this.reduceOnly = reduceOnly;
        this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
        this.oracleConfig = {
            confFilter: I80F48.from(oracleConfig.confFilter),
            maxStalenessSlots: oracleConfig.maxStalenessSlots,
        };
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
    get price() {
        if (!this._price) {
            throw new Error(`Undefined price for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`);
        }
        return this._price;
    }
    get uiPrice() {
        if (!this._uiPrice) {
            throw new Error(`Undefined price for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`);
        }
        return this._uiPrice;
    }
    get oracleLastUpdatedSlot() {
        if (!this._oracleLastUpdatedSlot) {
            throw new Error(`Undefined oracleLastUpdatedSlot for perpMarket ${this.publicKey} with marketIndex ${this.perpMarketIndex}!`);
        }
        return this._oracleLastUpdatedSlot;
    }
    get minOrderSize() {
        return this.baseLotsToUiConverter;
    }
    get tickSize() {
        return this.priceLotsToUiConverter;
    }
    insidePriceLimit(side, orderPrice) {
        return ((side === PerpOrderSide.bid &&
            orderPrice <= this.maintBaseLiabWeight.toNumber() * this.uiPrice) ||
            (side === PerpOrderSide.ask &&
                orderPrice >= this.maintBaseAssetWeight.toNumber() * this.uiPrice));
    }
    async loadAsks(client, forceReload = false) {
        if (forceReload || !this._asks) {
            const asks = await client.program.account.bookSide.fetch(this.asks);
            this._asks = BookSide.from(client, this, BookSideType.asks, asks);
        }
        return this._asks;
    }
    async loadBids(client, forceReload = false) {
        if (forceReload || !this._bids) {
            const bids = await client.program.account.bookSide.fetch(this.bids);
            this._bids = BookSide.from(client, this, BookSideType.bids, bids);
        }
        return this._bids;
    }
    async loadEventQueue(client) {
        const eventQueue = await client.program.account.eventQueue.fetch(this.eventQueue);
        return new PerpEventQueue(client, eventQueue.header, eventQueue.buf);
    }
    async loadFills(client, lastSeqNum = new BN(0)) {
        const eventQueue = await this.loadEventQueue(client);
        return eventQueue
            .eventsSince(lastSeqNum)
            .filter((event) => event.eventType == PerpEventQueue.FILL_EVENT_TYPE)
            .map(this.parseFillEvent.bind(this));
    }
    parseFillEvent(event) {
        const quantity = this.baseLotsToUi(event.quantity);
        const price = this.priceLotsToUi(event.price);
        return {
            ...event,
            quantity,
            size: quantity,
            price,
        };
    }
    async logOb(client) {
        let res = ``;
        res += `  ${this.name} OrderBook`;
        let orders = await this?.loadAsks(client);
        for (const order of orders.items()) {
            res += `\n ${order.uiPrice.toFixed(5).padStart(10)}, ${order.uiSize
                .toString()
                .padStart(10)} ${order.isOraclePegged && order.oraclePeggedProperties
                ? order.oraclePeggedProperties.pegLimit.toNumber() + ' (PegLimit)'
                : ''}`;
        }
        res += `\n  asks ↑ --------- ↓ bids`;
        orders = await this?.loadBids(client);
        for (const order of orders.items()) {
            res += `\n  ${order.uiPrice.toFixed(5).padStart(10)}, ${order.uiSize
                .toString()
                .padStart(10)} ${order.isOraclePegged && order.oraclePeggedProperties
                ? order.oraclePeggedProperties.pegLimit.toNumber() + ' (PegLimit)'
                : ''}`;
        }
        return res;
    }
    /**
     *
     * @param bids
     * @param asks
     * @returns returns funding rate per hour
     */
    getCurrentFundingRate(bids, asks) {
        const MIN_FUNDING = this.minFunding.toNumber();
        const MAX_FUNDING = this.maxFunding.toNumber();
        const bid = bids.getImpactPriceUi(new BN(this.impactQuantity));
        const ask = asks.getImpactPriceUi(new BN(this.impactQuantity));
        const indexPrice = this._uiPrice;
        let funding;
        if (bid !== undefined && ask !== undefined) {
            const bookPrice = (bid + ask) / 2;
            funding = Math.min(Math.max(bookPrice / indexPrice - 1, MIN_FUNDING), MAX_FUNDING);
        }
        else if (bid !== undefined) {
            funding = MAX_FUNDING;
        }
        else if (ask !== undefined) {
            funding = MIN_FUNDING;
        }
        else {
            funding = 0;
        }
        return funding / 24 / Math.pow(10, QUOTE_DECIMALS);
    }
    uiPriceToLots(price) {
        return toNative(price, QUOTE_DECIMALS)
            .mul(this.baseLotSize)
            .div(this.quoteLotSize.mul(new BN(Math.pow(10, this.baseDecimals))));
    }
    uiBaseToLots(quantity) {
        return toNative(quantity, this.baseDecimals).div(this.baseLotSize);
    }
    uiQuoteToLots(uiQuote) {
        return toNative(uiQuote, QUOTE_DECIMALS).div(this.quoteLotSize);
    }
    priceLotsToUi(price) {
        return parseFloat(price.toString()) * this.priceLotsToUiConverter;
    }
    priceNativeToUi(price) {
        return toUiDecimals(price, QUOTE_DECIMALS - this.baseDecimals);
    }
    baseLotsToUi(quantity) {
        return parseFloat(quantity.toString()) * this.baseLotsToUiConverter;
    }
    quoteLotsToUi(quantity) {
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
    async getSettlePnlCandidates(client, group, direction, count = 2) {
        let accountsWithSettleablePnl = (await client.getAllMangoAccounts(group, true))
            .filter((acc) => acc.perpPositionExistsForMarket(this))
            .map((acc) => {
            const pp = acc
                .perpActive()
                .find((pp) => pp.marketIndex === this.perpMarketIndex);
            return {
                account: acc,
                settleablePnl: pp.getSettleablePnl(group, this, acc),
            };
        });
        accountsWithSettleablePnl = accountsWithSettleablePnl
            .filter((acc) => 
        // need perp positions with -ve pnl to settle +ve pnl and vice versa
        (direction === 'negative' && acc.settleablePnl.lt(ZERO_I80F48())) ||
            (direction === 'positive' && acc.settleablePnl.gt(ZERO_I80F48())))
            .sort((a, b) => direction === 'negative'
            ? // most negative
                a.settleablePnl.cmp(b.settleablePnl)
            : // most positive
                b.settleablePnl.cmp(a.settleablePnl));
        if (direction === 'negative') {
            let stable = 0;
            for (let i = 0; i < accountsWithSettleablePnl.length; i++) {
                const acc = accountsWithSettleablePnl[i];
                const nextPnl = i + 1 < accountsWithSettleablePnl.length
                    ? accountsWithSettleablePnl[i + 1].settleablePnl
                    : ZERO_I80F48();
                const perpSettleHealth = acc.account.getPerpSettleHealth(group);
                acc.settleablePnl =
                    // need positive settle health to settle against +ve pnl
                    perpSettleHealth.gt(ZERO_I80F48())
                        ? // can only settle min
                            acc.settleablePnl.max(perpSettleHealth.neg())
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
        accountsWithSettleablePnl.sort((a, b) => direction === 'negative'
            ? // most negative
                a.settleablePnl.cmp(b.settleablePnl)
            : // most positive
                b.settleablePnl.cmp(a.settleablePnl));
        return accountsWithSettleablePnl.slice(0, count);
    }
    toString() {
        return ('PerpMarket ' +
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
            this.takerFee.toString());
    }
}
export class BookSide {
    client;
    perpMarket;
    type;
    rootFixed;
    rootOraclePegged;
    orderTreeNodes;
    static INNER_NODE_TAG = 1;
    static LEAF_NODE_TAG = 2;
    now;
    static from(client, perpMarket, bookSideType, obj) {
        return new BookSide(client, perpMarket, bookSideType, obj.roots[0], obj.roots[1], obj.nodes);
    }
    constructor(client, perpMarket, type, rootFixed, rootOraclePegged, orderTreeNodes, maxBookDelay) {
        this.client = client;
        this.perpMarket = perpMarket;
        this.type = type;
        this.rootFixed = rootFixed;
        this.rootOraclePegged = rootOraclePegged;
        this.orderTreeNodes = orderTreeNodes;
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
    static getPriceFromKey(key) {
        return key.ushrn(64);
    }
    /**
     * iterates over all orders
     */
    *items() {
        function isBetter(type, a, b) {
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
                }
                else {
                    yield oPegOrderRes.value;
                    oPegOrderRes = oPegGen.next();
                }
            }
            else if (fOrderRes.value && !oPegOrderRes.value) {
                yield fOrderRes.value;
                fOrderRes = fGen.next();
            }
            else if (!fOrderRes.value && oPegOrderRes.value) {
                yield oPegOrderRes.value;
                oPegOrderRes = oPegGen.next();
            }
            else if (!fOrderRes.value && !oPegOrderRes.value) {
                break;
            }
        }
    }
    /**
     * iterates over all orders,
     * skips oracle pegged orders which are invalid due to oracle price crossing the peg limit,
     * skips tif orders which are invalid due to tif having elapsed,
     */
    *itemsValid() {
        const itemsGen = this.items();
        let itemsRes = itemsGen.next();
        while (true) {
            if (itemsRes.value) {
                const val = itemsRes.value;
                if (!val.isExpired &&
                    (!val.isOraclePegged ||
                        (val.isOraclePegged && !val.oraclePeggedProperties.isInvalid))) {
                    yield val;
                }
                itemsRes = itemsGen.next();
            }
            else {
                break;
            }
        }
    }
    *fixedItems() {
        if (this.rootFixed.leafCount === 0) {
            return;
        }
        const now = this.now;
        const stack = [this.rootFixed.maybeNode];
        const [left, right] = this.type === BookSideType.bids ? [1, 0] : [0, 1];
        while (stack.length > 0) {
            const index = stack.pop();
            const node = this.orderTreeNodes.nodes[index];
            if (node.tag === BookSide.INNER_NODE_TAG) {
                const innerNode = BookSide.toInnerNode(this.client, node.data);
                stack.push(innerNode.children[right], innerNode.children[left]);
            }
            else if (node.tag === BookSide.LEAF_NODE_TAG) {
                const leafNode = BookSide.toLeafNode(this.client, node.data);
                const expiryTimestamp = leafNode.timeInForce
                    ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
                    : U64_MAX_BN;
                yield PerpOrder.from(this.perpMarket, leafNode, this.type, now.gt(expiryTimestamp));
            }
        }
    }
    *oraclePeggedItems() {
        if (this.rootOraclePegged.leafCount === 0) {
            return;
        }
        const now = this.now;
        const stack = [this.rootOraclePegged.maybeNode];
        const [left, right] = this.type === BookSideType.bids ? [1, 0] : [0, 1];
        while (stack.length > 0) {
            const index = stack.pop();
            const node = this.orderTreeNodes.nodes[index];
            if (node.tag === BookSide.INNER_NODE_TAG) {
                const innerNode = BookSide.toInnerNode(this.client, node.data);
                stack.push(innerNode.children[right], innerNode.children[left]);
            }
            else if (node.tag === BookSide.LEAF_NODE_TAG) {
                const leafNode = BookSide.toLeafNode(this.client, node.data);
                const expiryTimestamp = leafNode.timeInForce
                    ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
                    : U64_MAX_BN;
                yield PerpOrder.from(this.perpMarket, leafNode, this.type, now.gt(expiryTimestamp), true);
            }
        }
    }
    best() {
        return this.items().next().value;
    }
    getImpactPriceUi(baseLots) {
        const s = new BN(0);
        for (const order of this.items()) {
            s.iadd(order.sizeLots);
            if (s.gte(baseLots)) {
                return order.uiPrice;
            }
        }
        return undefined;
    }
    getL2(depth) {
        const levels = [];
        for (const { priceLots, sizeLots } of this.items()) {
            if (levels.length > 0 && levels[levels.length - 1][0].eq(priceLots)) {
                levels[levels.length - 1][1].iadd(sizeLots);
            }
            else if (levels.length === depth) {
                break;
            }
            else {
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
    getL2Ui(depth) {
        const levels = [];
        for (const { uiPrice: price, uiSize: size } of this.items()) {
            if (levels.length > 0 && levels[levels.length - 1][0] === price) {
                levels[levels.length - 1][1] += size;
            }
            else if (levels.length === depth) {
                break;
            }
            else {
                levels.push([price, size]);
            }
        }
        return levels;
    }
    static toInnerNode(client, data) {
        return client.program._coder.types.typeLayouts
            .get('InnerNode')
            .decode(Buffer.from([BookSide.INNER_NODE_TAG].concat(data)));
    }
    static toLeafNode(client, data) {
        return LeafNode.from(client.program._coder.types.typeLayouts
            .get('LeafNode')
            .decode(Buffer.from([BookSide.LEAF_NODE_TAG].concat(data))));
    }
}
export class BookSideType {
    static bids = { bids: {} };
    static asks = { asks: {} };
}
export class LeafNode {
    ownerSlot;
    orderType;
    timeInForce;
    key;
    owner;
    quantity;
    timestamp;
    pegLimit;
    static from(obj) {
        return new LeafNode(obj.ownerSlot, obj.orderType, obj.timeInForce, obj.key, obj.owner, obj.quantity, obj.timestamp, obj.pegLimit);
    }
    constructor(ownerSlot, orderType, timeInForce, key, owner, quantity, timestamp, pegLimit) {
        this.ownerSlot = ownerSlot;
        this.orderType = orderType;
        this.timeInForce = timeInForce;
        this.key = key;
        this.owner = owner;
        this.quantity = quantity;
        this.timestamp = timestamp;
        this.pegLimit = pegLimit;
    }
}
export class InnerNode {
    children;
    static from(obj) {
        return new InnerNode(obj.children);
    }
    constructor(children) {
        this.children = children;
    }
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
    seqNum;
    orderId;
    owner;
    openOrdersSlot;
    feeTier;
    uiPrice;
    priceLots;
    uiSize;
    sizeLots;
    side;
    timestamp;
    expiryTimestamp;
    perpMarketIndex;
    isExpired;
    isOraclePegged;
    oraclePeggedProperties;
    static from(perpMarket, leafNode, type, isExpired = false, isOraclePegged = false) {
        const side = type == BookSideType.bids ? PerpOrderSide.bid : PerpOrderSide.ask;
        let priceLots;
        let oraclePeggedProperties;
        if (isOraclePegged) {
            const priceData = leafNode.key.ushrn(64);
            const priceOffset = priceData.sub(new BN(1).ushln(63));
            priceLots = perpMarket.uiPriceToLots(perpMarket.uiPrice).add(priceOffset);
            const isInvalid = type === BookSideType.bids
                ? priceLots.gt(leafNode.pegLimit)
                : leafNode.pegLimit.gt(priceLots);
            oraclePeggedProperties = {
                isInvalid,
                priceOffset,
                uiPriceOffset: perpMarket.priceLotsToUi(priceOffset),
                pegLimit: leafNode.pegLimit,
                uiPegLimit: perpMarket.priceLotsToUi(leafNode.pegLimit),
            };
        }
        else {
            priceLots = BookSide.getPriceFromKey(leafNode.key);
        }
        const expiryTimestamp = leafNode.timeInForce
            ? leafNode.timestamp.add(new BN(leafNode.timeInForce))
            : U64_MAX_BN;
        return new PerpOrder(type === BookSideType.bids
            ? RUST_U64_MAX().sub(leafNode.key.maskn(64))
            : leafNode.key.maskn(64), leafNode.key, leafNode.owner, leafNode.ownerSlot, 0, perpMarket.priceLotsToUi(priceLots), priceLots, perpMarket.baseLotsToUi(leafNode.quantity), leafNode.quantity, side, leafNode.timestamp, expiryTimestamp, perpMarket.perpMarketIndex, isExpired, isOraclePegged, oraclePeggedProperties);
    }
    constructor(seqNum, orderId, owner, openOrdersSlot, feeTier, uiPrice, priceLots, uiSize, sizeLots, side, timestamp, expiryTimestamp, perpMarketIndex, isExpired = false, isOraclePegged = false, oraclePeggedProperties) {
        this.seqNum = seqNum;
        this.orderId = orderId;
        this.owner = owner;
        this.openOrdersSlot = openOrdersSlot;
        this.feeTier = feeTier;
        this.uiPrice = uiPrice;
        this.priceLots = priceLots;
        this.uiSize = uiSize;
        this.sizeLots = sizeLots;
        this.side = side;
        this.timestamp = timestamp;
        this.expiryTimestamp = expiryTimestamp;
        this.perpMarketIndex = perpMarketIndex;
        this.isExpired = isExpired;
        this.isOraclePegged = isOraclePegged;
        this.oraclePeggedProperties = oraclePeggedProperties;
    }
    get price() {
        return this.uiPrice;
    }
    get size() {
        return this.uiSize;
    }
}
export class PerpEventQueue {
    static FILL_EVENT_TYPE = 0;
    static OUT_EVENT_TYPE = 1;
    static LIQUIDATE_EVENT_TYPE = 2;
    head;
    count;
    seqNum;
    rawEvents;
    constructor(client, header, buf) {
        this.head = header.head;
        this.count = header.count;
        this.seqNum = header.seqNum;
        this.rawEvents = buf.map((event) => {
            if (event.eventType === PerpEventQueue.FILL_EVENT_TYPE) {
                return client.program._coder.types.typeLayouts
                    .get('FillEvent')
                    .decode(Buffer.from([PerpEventQueue.FILL_EVENT_TYPE].concat(event.padding)));
            }
            else if (event.eventType === PerpEventQueue.OUT_EVENT_TYPE) {
                return client.program._coder.types.typeLayouts
                    .get('OutEvent')
                    .decode(Buffer.from([PerpEventQueue.OUT_EVENT_TYPE].concat(event.padding)));
            }
            else if (event.eventType === PerpEventQueue.LIQUIDATE_EVENT_TYPE) {
                return client.program._coder.types.typeLayouts
                    .get('LiquidateEvent')
                    .decode(Buffer.from([PerpEventQueue.LIQUIDATE_EVENT_TYPE].concat(event.padding)));
            }
            throw new Error(`Unknown event with eventType ${event.eventType}!`);
        });
    }
    getUnconsumedEvents() {
        const events = [];
        const head = this.head;
        for (let i = 0; i < this.count; i++) {
            events.push(this.rawEvents[(head + i) % this.rawEvents.length]);
        }
        return events;
    }
    eventsSince(lastSeqNum) {
        return this.rawEvents
            .filter((e) => e.seqNum.gt(lastSeqNum === undefined ? new BN(0) : lastSeqNum))
            .sort((a, b) => a.seqNum.cmp(b.seqNum));
    }
}
