import { BN } from '@coral-xyz/anchor';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { OpenOrders, Orderbook } from '@project-serum/serum/lib/market';
import { OPENBOOK_PROGRAM_ID, RUST_I64_MAX, RUST_I64_MIN } from '../constants';
import { I80F48, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { toNativeI80F48, toUiDecimals, toUiDecimalsForQuote } from '../utils';
import { HealthCache } from './healthCache';
import { PerpOrderSide } from './perp';
import { Serum3Side } from './serum3';
export class MangoAccount {
    publicKey;
    group;
    owner;
    delegate;
    accountNum;
    beingLiquidated;
    inHealthRegion;
    netDeposits;
    perpSpotTransfers;
    healthRegionBeginInitHealth;
    frozenUntil;
    buybackFeesAccruedCurrent;
    buybackFeesAccruedPrevious;
    buybackFeesExpiryTimestamp;
    headerVersion;
    serum3OosMapByMarketIndex;
    name;
    tokens;
    serum3;
    perps;
    perpOpenOrders;
    static from(publicKey, obj) {
        return new MangoAccount(publicKey, obj.group, obj.owner, obj.name, obj.delegate, obj.accountNum, obj.beingLiquidated == 1, obj.inHealthRegion == 1, obj.netDeposits, obj.perpSpotTransfers, obj.healthRegionBeginInitHealth, obj.frozenUntil, obj.buybackFeesAccruedCurrent, obj.buybackFeesAccruedPrevious, obj.buybackFeesExpiryTimestamp, obj.headerVersion, obj.tokens, obj.serum3, obj.perps, obj.perpOpenOrders, new Map());
    }
    constructor(publicKey, group, owner, name, delegate, accountNum, beingLiquidated, inHealthRegion, netDeposits, perpSpotTransfers, healthRegionBeginInitHealth, frozenUntil, buybackFeesAccruedCurrent, buybackFeesAccruedPrevious, buybackFeesExpiryTimestamp, headerVersion, tokens, serum3, perps, perpOpenOrders, serum3OosMapByMarketIndex) {
        this.publicKey = publicKey;
        this.group = group;
        this.owner = owner;
        this.delegate = delegate;
        this.accountNum = accountNum;
        this.beingLiquidated = beingLiquidated;
        this.inHealthRegion = inHealthRegion;
        this.netDeposits = netDeposits;
        this.perpSpotTransfers = perpSpotTransfers;
        this.healthRegionBeginInitHealth = healthRegionBeginInitHealth;
        this.frozenUntil = frozenUntil;
        this.buybackFeesAccruedCurrent = buybackFeesAccruedCurrent;
        this.buybackFeesAccruedPrevious = buybackFeesAccruedPrevious;
        this.buybackFeesExpiryTimestamp = buybackFeesExpiryTimestamp;
        this.headerVersion = headerVersion;
        this.serum3OosMapByMarketIndex = serum3OosMapByMarketIndex;
        this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
        this.tokens = tokens.map((dto) => TokenPosition.from(dto));
        this.serum3 = serum3.map((dto) => Serum3Orders.from(dto));
        this.perps = perps.map((dto) => PerpPosition.from(dto));
        this.perpOpenOrders = perpOpenOrders.map((dto) => PerpOo.from(dto));
    }
    async reload(client) {
        const mangoAccount = await client.getMangoAccount(this);
        await mangoAccount.reloadSerum3OpenOrders(client);
        Object.assign(this, mangoAccount);
        return mangoAccount;
    }
    async reloadWithSlot(client) {
        const resp = await client.getMangoAccountWithSlot(this.publicKey);
        await resp?.value.reloadSerum3OpenOrders(client);
        Object.assign(this, resp?.value);
        return { value: resp.value, slot: resp.slot };
    }
    async reloadSerum3OpenOrders(client) {
        const serum3Active = this.serum3Active();
        const ais = await client.program.provider.connection.getMultipleAccountsInfo(serum3Active.map((serum3) => serum3.openOrders));
        this.serum3OosMapByMarketIndex = new Map(Array.from(ais.map((ai, i) => {
            if (!ai) {
                throw new Error(`Undefined AI for open orders ${serum3Active[i].openOrders} and market ${serum3Active[i].marketIndex}!`);
            }
            const oo = OpenOrders.fromAccountInfo(serum3Active[i].openOrders, ai, OPENBOOK_PROGRAM_ID[client.cluster]);
            return [serum3Active[i].marketIndex, oo];
        })));
        return this;
    }
    isDelegate(client) {
        return this.delegate.equals(client.program.provider.wallet.publicKey);
    }
    isOperational() {
        return this.frozenUntil.lt(new BN(Date.now() / 1000));
    }
    tokensActive() {
        return this.tokens.filter((token) => token.isActive());
    }
    serum3Active() {
        return this.serum3.filter((serum3) => serum3.isActive());
    }
    perpPositionExistsForMarket(perpMarket) {
        return this.perps.some((pp) => pp.isActive() && pp.marketIndex == perpMarket.perpMarketIndex);
    }
    perpOrderExistsForMarket(perpMarket) {
        return this.perpOpenOrders.some((poo) => poo.isActive() && poo.orderMarket == perpMarket.perpMarketIndex);
    }
    perpActive() {
        return this.perps.filter((perp) => perp.isActive());
    }
    perpOrdersActive() {
        return this.perpOpenOrders.filter((oo) => oo.orderMarket !== PerpOo.OrderMarketUnset);
    }
    getToken(tokenIndex) {
        return this.tokens.find((ta) => ta.tokenIndex == tokenIndex);
    }
    getSerum3Account(marketIndex) {
        return this.serum3.find((sa) => sa.marketIndex == marketIndex);
    }
    getPerpPosition(perpMarketIndex) {
        return this.perps.find((pp) => pp.marketIndex == perpMarketIndex);
    }
    getPerpPositionUi(group, perpMarketIndex, useEventQueue) {
        const pp = this.perps.find((pp) => pp.marketIndex == perpMarketIndex);
        if (!pp) {
            throw new Error(`No position found for PerpMarket ${perpMarketIndex}!`);
        }
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        return pp.getBasePositionUi(perpMarket, useEventQueue);
    }
    getSerum3OoAccount(marketIndex) {
        const oo = this.serum3OosMapByMarketIndex.get(marketIndex);
        if (!oo) {
            throw new Error(`Open orders account not loaded for market with marketIndex ${marketIndex}!`);
        }
        return oo;
    }
    // How to navigate
    // * if a function is returning a I80F48, then usually the return value is in native quote or native token, unless specified
    // * if a function is returning a number, then usually the return value is in ui tokens, unless specified
    // * functions try to be explicit by having native or ui in the name to better reflect the value
    // * some values might appear unexpected large or small, usually the doc contains a "note"
    /**
     *
     * @param bank
     * @returns native balance for a token, is signed
     */
    getTokenBalance(bank) {
        const tp = this.getToken(bank.tokenIndex);
        return tp ? tp.balance(bank) : ZERO_I80F48();
    }
    /**
     *
     * @param bank
     * @returns native deposits for a token, 0 if position has borrows
     */
    getTokenDeposits(bank) {
        const tp = this.getToken(bank.tokenIndex);
        return tp ? tp.deposits(bank) : ZERO_I80F48();
    }
    /**
     *
     * @param bank
     * @returns native borrows for a token, 0 if position has deposits
     */
    getTokenBorrows(bank) {
        const tp = this.getToken(bank.tokenIndex);
        return tp ? tp.borrows(bank) : ZERO_I80F48();
    }
    /**
     *
     * @param bank
     * @returns UI balance for a token, is signed
     */
    getTokenBalanceUi(bank) {
        const tp = this.getToken(bank.tokenIndex);
        return tp ? tp.balanceUi(bank) : 0;
    }
    /**
     *
     * @param bank
     * @returns UI deposits for a token, 0 or more
     */
    getTokenDepositsUi(bank) {
        const ta = this.getToken(bank.tokenIndex);
        return ta ? ta.depositsUi(bank) : 0;
    }
    /**
     *
     * @param bank
     * @returns UI borrows for a token, 0 or less
     */
    getTokenBorrowsUi(bank) {
        const ta = this.getToken(bank.tokenIndex);
        return ta ? ta.borrowsUi(bank) : 0;
    }
    /**
     * Health, see health.rs or https://docs.mango.markets/mango-markets/health-overview
     * @param healthType
     * @returns raw health number, in native quote
     */
    getHealth(group, healthType) {
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc.health(healthType);
    }
    getPerpSettleHealth(group) {
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc.perpSettleHealth();
    }
    /**
     * Health ratio, which is computed so `100 * (assets-liabs)/liabs`
     * Note: health ratio is technically ∞ if liabs are 0
     * @param healthType
     * @returns health ratio, in percentage form
     */
    getHealthRatio(group, healthType) {
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc.healthRatio(healthType);
    }
    /**
     * Health ratio
     * @param healthType
     * @returns health ratio, in percentage form, capped to 100
     */
    getHealthRatioUi(group, healthType) {
        const ratio = this.getHealthRatio(group, healthType).toNumber();
        return ratio > 100 ? 100 : Math.trunc(ratio);
    }
    /**
     * Sum of all the assets i.e. token deposits, borrows, total assets in spot open orders, and perps positions.
     * @returns equity, in native quote
     */
    getEquity(group) {
        const tokensMap = new Map();
        for (const tp of this.tokensActive()) {
            const bank = group.getFirstBankByTokenIndex(tp.tokenIndex);
            tokensMap.set(tp.tokenIndex, tp.balance(bank).mul(bank.price));
        }
        for (const sp of this.serum3Active()) {
            const oo = this.getSerum3OoAccount(sp.marketIndex);
            const baseBank = group.getFirstBankByTokenIndex(sp.baseTokenIndex);
            tokensMap
                .get(baseBank.tokenIndex)
                .iadd(I80F48.fromI64(oo.baseTokenTotal).mul(baseBank.price));
            const quoteBank = group.getFirstBankByTokenIndex(sp.quoteTokenIndex);
            // NOTE: referrerRebatesAccrued is not declared on oo class, but the layout
            // is aware of it
            tokensMap
                .get(baseBank.tokenIndex)
                .iadd(I80F48.fromI64(oo.quoteTokenTotal.add(oo.referrerRebatesAccrued)).mul(quoteBank.price));
        }
        const tokenEquity = Array.from(tokensMap.values()).reduce((a, b) => a.add(b), ZERO_I80F48());
        const perpEquity = this.perpActive().reduce((a, b) => a.add(b.getEquity(group.getPerpMarketByMarketIndex(b.marketIndex))), ZERO_I80F48());
        return tokenEquity.add(perpEquity);
    }
    /**
     * The amount of native quote you could withdraw against your existing assets.
     * @returns collateral value, in native quote
     */
    getCollateralValue(group) {
        return this.getHealth(group, HealthType.init);
    }
    /**
     * Sum of all positive assets.
     * @returns assets, in native quote
     */
    getAssetsValue(group, healthType) {
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc.assets(healthType);
    }
    /**
     * Sum of all negative assets.
     * @returns liabs, in native quote
     */
    getLiabsValue(group, healthType) {
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc.liabs(healthType);
    }
    /**
     * @returns Overall PNL, in native quote
     * PNL is defined here as spot value + serum3 open orders value + perp value - net deposits value (evaluated at native quote price at the time of the deposit/withdraw)
     * spot value + serum3 open orders value + perp value is returned by getEquity (open orders values are added to spot token values implicitly)
     */
    getPnl(group) {
        return this.getEquity(group)?.add(I80F48.fromI64(this.netDeposits).mul(I80F48.fromNumber(-1)));
    }
    /**
     * @returns token cumulative interest, in native token units. Sum of deposit and borrow interest.
     * Caveat: This will only return cumulative interest since the tokenPosition was last opened.
     * If the tokenPosition was closed and reopened multiple times it is necessary to add this result to
     * cumulative interest at each of the prior tokenPosition closings (from mango API) to get the all time
     * cumulative interest.
     */
    getCumulativeInterest(bank) {
        const token = this.getToken(bank.tokenIndex);
        if (token === undefined) {
            // tokenPosition does not exist on mangoAccount so no cumulative interest
            return 0;
        }
        else {
            if (token.indexedPosition.isPos()) {
                const interest = bank.depositIndex
                    .sub(token.previousIndex)
                    .mul(token.indexedPosition)
                    .toNumber();
                return (interest +
                    token.cumulativeDepositInterest +
                    token.cumulativeBorrowInterest);
            }
            else {
                const interest = bank.borrowIndex
                    .sub(token.previousIndex)
                    .mul(token.indexedPosition)
                    .toNumber();
                return (interest +
                    token.cumulativeDepositInterest +
                    token.cumulativeBorrowInterest);
            }
        }
    }
    /**
     * The amount of given native token you can withdraw including borrows, considering all existing assets as collateral.
     * @returns amount of given native token you can borrow, considering all existing assets as collateral, in native token
     *
     * TODO: take into account net_borrow_limit and min_vault_to_deposits_ratio
     */
    getMaxWithdrawWithBorrowForToken(group, mintPk) {
        const tokenBank = group.getFirstBankByMint(mintPk);
        const initHealth = this.getHealth(group, HealthType.init);
        // Case 1:
        // Cannot withdraw if init health is below 0
        if (initHealth.lte(ZERO_I80F48())) {
            return ZERO_I80F48();
        }
        // Deposits need special treatment since they would neither count towards liabilities
        // nor would be charged loanOriginationFeeRate when withdrawn
        const tp = this.getToken(tokenBank.tokenIndex);
        const existingTokenDeposits = tp ? tp.deposits(tokenBank) : ZERO_I80F48();
        let existingPositionHealthContrib = ZERO_I80F48();
        if (existingTokenDeposits.gt(ZERO_I80F48())) {
            existingPositionHealthContrib = existingTokenDeposits
                .mul(tokenBank.price)
                .imul(tokenBank.initAssetWeight);
        }
        // Case 2: token deposits have higher contribution than initHealth,
        // can withdraw without borrowing until initHealth reaches 0
        if (existingPositionHealthContrib.gt(initHealth)) {
            const withdrawAbleExistingPositionHealthContrib = initHealth;
            return withdrawAbleExistingPositionHealthContrib
                .div(tokenBank.initAssetWeight)
                .div(tokenBank.price);
        }
        // Case 3: withdraw = withdraw existing deposits + borrows until initHealth reaches 0
        const initHealthWithoutExistingPosition = initHealth.sub(existingPositionHealthContrib);
        let maxBorrowNative = initHealthWithoutExistingPosition
            .div(tokenBank.initLiabWeight)
            .div(tokenBank.price);
        // Cap maxBorrow to maintain minVaultToDepositsRatio on the bank
        const vaultAmount = group.vaultAmountsMap.get(tokenBank.vault.toBase58());
        if (!vaultAmount) {
            throw new Error(`No vault amount found for ${tokenBank.name} vault ${tokenBank.vault}!`);
        }
        const vaultAmountAfterWithdrawingDeposits = I80F48.fromU64(vaultAmount).sub(existingTokenDeposits);
        const expectedVaultMinAmount = tokenBank
            .nativeDeposits()
            .mul(I80F48.fromNumber(tokenBank.minVaultToDepositsRatio));
        if (vaultAmountAfterWithdrawingDeposits.gt(expectedVaultMinAmount)) {
            maxBorrowNative = maxBorrowNative.min(vaultAmountAfterWithdrawingDeposits.sub(expectedVaultMinAmount));
        }
        const maxBorrowNativeWithoutFees = maxBorrowNative.div(ONE_I80F48().add(tokenBank.loanOriginationFeeRate));
        return maxBorrowNativeWithoutFees.add(existingTokenDeposits);
    }
    getMaxWithdrawWithBorrowForTokenUi(group, mintPk) {
        const maxWithdrawWithBorrow = this.getMaxWithdrawWithBorrowForToken(group, mintPk);
        return toUiDecimals(maxWithdrawWithBorrow, group.getMintDecimals(mintPk));
    }
    /**
     * The max amount of given source ui token you can swap to a target token.
     * @returns max amount of given source ui token you can swap to a target token, in ui token
     */
    getMaxSourceUiForTokenSwap(group, sourceMintPk, targetMintPk, slippageAndFeesFactor = 1) {
        if (sourceMintPk.equals(targetMintPk)) {
            return 0;
        }
        const sourceBank = group.getFirstBankByMint(sourceMintPk);
        const targetBank = group.getFirstBankByMint(targetMintPk);
        const hc = HealthCache.fromMangoAccount(group, this);
        let maxSource = hc.getMaxSwapSource(sourceBank, targetBank, I80F48.fromNumber(slippageAndFeesFactor *
            ((sourceBank.uiPrice / targetBank.uiPrice) *
                Math.pow(10, targetBank.mintDecimals - sourceBank.mintDecimals))));
        const sourceBalance = this.getTokenBalance(sourceBank);
        if (maxSource.gt(sourceBalance)) {
            const sourceBorrow = maxSource.sub(sourceBalance);
            maxSource = sourceBalance.add(sourceBorrow.div(ONE_I80F48().add(sourceBank.loanOriginationFeeRate)));
        }
        return toUiDecimals(maxSource, group.getMintDecimals(sourceMintPk));
    }
    /**
     * Simulates new health ratio after applying tokenChanges to the token positions.
     * Note: token changes are expected in ui amounts
     *
     * e.g. useful to simulate health after a potential swap.
     * Note: health ratio is technically ∞ if liabs are 0
     * @returns health ratio, in percentage form
     */
    simHealthRatioWithTokenPositionUiChanges(group, uiTokenChanges, healthType = HealthType.init) {
        const nativeTokenChanges = uiTokenChanges.map((tokenChange) => {
            return {
                nativeTokenAmount: toNativeI80F48(tokenChange.uiTokenAmount, group.getMintDecimals(tokenChange.mintPk)),
                mintPk: tokenChange.mintPk,
            };
        });
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc
            .simHealthRatioWithTokenPositionChanges(group, nativeTokenChanges, healthType)
            .toNumber();
    }
    async loadSerum3OpenOrdersAccounts(client) {
        const openOrderPks = this.serum3Active().map((s) => s.openOrders);
        if (!openOrderPks.length)
            return [];
        const response = await client.program.provider.connection.getMultipleAccountsInfo(openOrderPks);
        const accounts = response.filter((a) => Boolean(a));
        return accounts.map((acc, index) => {
            return OpenOrders.fromAccountInfo(this.serum3[index].openOrders, acc, OPENBOOK_PROGRAM_ID[client.cluster]);
        });
    }
    async loadSerum3OpenOrdersForMarket(client, group, externalMarketPk) {
        const serum3Market = group.getSerum3MarketByExternalMarket(externalMarketPk);
        const serum3OO = this.serum3Active().find((s) => s.marketIndex === serum3Market.marketIndex);
        if (!serum3OO) {
            throw new Error(`No open orders account found for ${externalMarketPk}`);
        }
        const serum3MarketExternal = group.serum3ExternalMarketsMap.get(externalMarketPk.toBase58());
        const [bidsInfo, asksInfo] = await client.program.provider.connection.getMultipleAccountsInfo([
            serum3MarketExternal.bidsAddress,
            serum3MarketExternal.asksAddress,
        ]);
        if (!bidsInfo) {
            throw new Error(`Undefined bidsInfo for serum3Market with externalMarket ${externalMarketPk.toString()}`);
        }
        if (!asksInfo) {
            throw new Error(`Undefined asksInfo for serum3Market with externalMarket ${externalMarketPk.toString()}`);
        }
        const bids = Orderbook.decode(serum3MarketExternal, bidsInfo.data);
        const asks = Orderbook.decode(serum3MarketExternal, asksInfo.data);
        return [...bids, ...asks].filter((o) => o.openOrdersAddress.equals(serum3OO.openOrders));
    }
    /**
     * TODO REWORK, know to break in binary search, also make work for limit orders
     *
     * @param group
     * @param externalMarketPk
     * @returns maximum ui quote which can be traded at oracle price for base token given current health
     */
    getMaxQuoteForSerum3BidUi(group, externalMarketPk) {
        const serum3Market = group.getSerum3MarketByExternalMarket(externalMarketPk);
        const baseBank = group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex);
        const quoteBank = group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        const nativeAmount = hc.getMaxSerum3OrderForHealthRatio(baseBank, quoteBank, serum3Market, Serum3Side.bid, I80F48.fromNumber(2));
        let quoteAmount = nativeAmount.div(quoteBank.price);
        // If its a bid then the reserved fund and potential loan is in base
        // also keep some buffer for fees, use taker fees for worst case simulation.
        const quoteBalance = this.getTokenBalance(quoteBank);
        if (quoteAmount.gt(quoteBalance)) {
            const quoteBorrow = quoteAmount.sub(quoteBalance);
            quoteAmount = quoteBalance.add(quoteBorrow.div(ONE_I80F48().add(quoteBank.loanOriginationFeeRate)));
        }
        quoteAmount = quoteAmount.div(ONE_I80F48().add(I80F48.fromNumber(serum3Market.getFeeRates(true))));
        return toUiDecimals(nativeAmount, quoteBank.mintDecimals);
    }
    /**
     * TODO REWORK, know to break in binary search, also make work for limit orders
     * @param group
     * @param externalMarketPk
     * @returns maximum ui base which can be traded at oracle price for quote token given current health
     */
    getMaxBaseForSerum3AskUi(group, externalMarketPk) {
        const serum3Market = group.getSerum3MarketByExternalMarket(externalMarketPk);
        const baseBank = group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex);
        const quoteBank = group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        const nativeAmount = hc.getMaxSerum3OrderForHealthRatio(baseBank, quoteBank, serum3Market, Serum3Side.ask, I80F48.fromNumber(2));
        let baseAmount = nativeAmount.div(baseBank.price);
        // If its a ask then the reserved fund and potential loan is in base
        // also keep some buffer for fees, use taker fees for worst case simulation.
        const baseBalance = this.getTokenBalance(baseBank);
        if (baseAmount.gt(baseBalance)) {
            const baseBorrow = baseAmount.sub(baseBalance);
            baseAmount = baseBalance.add(baseBorrow.div(ONE_I80F48().add(baseBank.loanOriginationFeeRate)));
        }
        baseAmount = baseAmount.div(ONE_I80F48().add(I80F48.fromNumber(serum3Market.getFeeRates(true))));
        return toUiDecimals(baseAmount, baseBank.mintDecimals);
    }
    /**
     *
     * @param group
     * @param uiQuoteAmount
     * @param externalMarketPk
     * @param healthType
     * @returns health ratio after a bid with uiQuoteAmount is placed
     */
    simHealthRatioWithSerum3BidUiChanges(group, uiQuoteAmount, externalMarketPk, healthType = HealthType.init) {
        const serum3Market = group.getSerum3MarketByExternalMarket(externalMarketPk);
        const baseBank = group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex);
        const quoteBank = group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc
            .simHealthRatioWithSerum3BidChanges(baseBank, quoteBank, toNativeI80F48(uiQuoteAmount, group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex)
            .mintDecimals), serum3Market, healthType)
            .toNumber();
    }
    /**
     *
     * @param group
     * @param uiBaseAmount
     * @param externalMarketPk
     * @param healthType
     * @returns health ratio after an ask with uiBaseAmount is placed
     */
    simHealthRatioWithSerum3AskUiChanges(group, uiBaseAmount, externalMarketPk, healthType = HealthType.init) {
        const serum3Market = group.getSerum3MarketByExternalMarket(externalMarketPk);
        const baseBank = group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex);
        const quoteBank = group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc
            .simHealthRatioWithSerum3AskChanges(baseBank, quoteBank, toNativeI80F48(uiBaseAmount, group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
            .mintDecimals), serum3Market, healthType)
            .toNumber();
    }
    // TODO: don't send a settle instruction if there's nothing to settle
    async serum3SettleFundsForAllMarkets(client, group) {
        // Future: collect ixs, batch them, and send them in fewer txs
        return await Promise.all(this.serum3Active().map((s) => {
            const serum3Market = group.getSerum3MarketByMarketIndex(s.marketIndex);
            return client.serum3SettleFunds(group, this, serum3Market.serumMarketExternal);
        }));
    }
    // TODO: cancel until all are cancelled
    async serum3CancelAllOrdersForAllMarkets(client, group) {
        // Future: collect ixs, batch them, and send them in in fewer txs
        return await Promise.all(this.serum3Active().map((s) => {
            const serum3Market = group.getSerum3MarketByMarketIndex(s.marketIndex);
            return client.serum3CancelAllOrders(group, this, serum3Market.serumMarketExternal);
        }));
    }
    /**
     * TODO: also think about limit orders
     *
     * The max ui quote you can place a market/ioc bid on the market,
     * price is the ui price at which you think the order would materialiase.
     * @param group
     * @param perpMarketName
     * @returns maximum ui quote which can be traded at oracle price for quote token given current health
     */
    getMaxQuoteForPerpBidUi(group, perpMarketIndex) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        const baseLots = hc.getMaxPerpForHealthRatio(perpMarket, I80F48.fromNumber(perpMarket.uiPrice), PerpOrderSide.bid, I80F48.fromNumber(2));
        const nativeBase = baseLots.mul(I80F48.fromI64(perpMarket.baseLotSize));
        const nativeQuote = nativeBase.mul(perpMarket.price);
        return toUiDecimalsForQuote(nativeQuote);
    }
    /**
     * TODO: also think about limit orders
     *
     * The max ui base you can place a market/ioc ask on the market,
     * price is the ui price at which you think the order would materialiase.
     * @param group
     * @param perpMarketName
     * @param uiPrice ui price at which ask would be placed at
     * @returns max ui base ask
     */
    getMaxBaseForPerpAskUi(group, perpMarketIndex) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        const baseLots = hc.getMaxPerpForHealthRatio(perpMarket, I80F48.fromNumber(perpMarket.uiPrice), PerpOrderSide.ask, I80F48.fromNumber(2));
        return perpMarket.baseLotsToUi(new BN(baseLots.toString()));
    }
    simHealthRatioWithPerpBidUiChanges(group, perpMarketIndex, size) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const pp = this.getPerpPosition(perpMarket.perpMarketIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc
            .simHealthRatioWithPerpOrderChanges(perpMarket, pp
            ? pp
            : PerpPosition.emptyFromPerpMarketIndex(perpMarket.perpMarketIndex), PerpOrderSide.bid, perpMarket.uiBaseToLots(size), I80F48.fromNumber(perpMarket.uiPrice), HealthType.init)
            .toNumber();
    }
    simHealthRatioWithPerpAskUiChanges(group, perpMarketIndex, size) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const pp = this.getPerpPosition(perpMarket.perpMarketIndex);
        const hc = HealthCache.fromMangoAccount(group, this);
        return hc
            .simHealthRatioWithPerpOrderChanges(perpMarket, pp
            ? pp
            : PerpPosition.emptyFromPerpMarketIndex(perpMarket.perpMarketIndex), PerpOrderSide.ask, perpMarket.uiBaseToLots(size), I80F48.fromNumber(perpMarket.uiPrice), HealthType.init)
            .toNumber();
    }
    async loadPerpOpenOrdersForMarket(client, group, perpMarketIndex, forceReload) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const [bids, asks] = await Promise.all([
            perpMarket.loadBids(client, forceReload),
            perpMarket.loadAsks(client, forceReload),
        ]);
        return [...bids.items(), ...asks.items()].filter((order) => order.owner.equals(this.publicKey));
    }
    getBuybackFeesAccrued() {
        return this.buybackFeesAccruedCurrent.add(this.buybackFeesAccruedPrevious);
    }
    getBuybackFeesAccruedUi() {
        return toUiDecimalsForQuote(this.getBuybackFeesAccrued());
    }
    getMaxFeesBuyback(group) {
        const mngoBalanceValueWithBonus = new BN(this.getTokenBalance(group.getFirstBankForMngo())
            .mul(group.getFirstBankForMngo().price)
            .mul(I80F48.fromNumber(group.buybackFeesMngoBonusFactor))
            .floor()
            .toNumber());
        return BN.max(BN.min(this.getBuybackFeesAccrued(), mngoBalanceValueWithBonus), new BN(0));
    }
    getMaxFeesBuybackUi(group) {
        return toUiDecimalsForQuote(this.getMaxFeesBuyback(group));
    }
    toString(group, onlyTokens = false) {
        let res = 'MangoAccount';
        res = res + '\n pk: ' + this.publicKey.toString();
        res = res + '\n name: ' + this.name;
        res = res + '\n accountNum: ' + this.accountNum;
        res = res + '\n owner: ' + this.owner;
        res = res + '\n delegate: ' + this.delegate;
        res =
            res +
                `\n max token slots ${this.tokens.length}, max serum3 slots ${this.serum3.length}, max perp slots ${this.perps.length}, max perp oo slots ${this.perpOpenOrders.length}`;
        res =
            this.tokensActive().length > 0
                ? res +
                    '\n tokens:' +
                    JSON.stringify(this.tokens
                        .filter((token, i) => token.isActive())
                        .map((token, i) => token.toString(group, i)), null, 4)
                : res + '';
        if (onlyTokens) {
            return res;
        }
        res =
            this.serum3Active().length > 0
                ? res + '\n serum:' + JSON.stringify(this.serum3Active(), null, 4)
                : res + '';
        res =
            this.perpActive().length > 0
                ? res +
                    '\n perps:' +
                    JSON.stringify(this.perpActive().map((p) => p.toString(group?.getPerpMarketByMarketIndex(p.marketIndex))), null, 4)
                : res + '';
        res =
            this.perpOrdersActive().length > 0
                ? res +
                    '\n perps oo:' +
                    JSON.stringify(this.perpOrdersActive(), null, 4)
                : res + '';
        return res;
    }
}
export class TokenPosition {
    indexedPosition;
    tokenIndex;
    inUseCount;
    previousIndex;
    cumulativeDepositInterest;
    cumulativeBorrowInterest;
    static TokenIndexUnset = 65535;
    static from(dto) {
        return new TokenPosition(I80F48.from(dto.indexedPosition), dto.tokenIndex, dto.inUseCount, I80F48.from(dto.previousIndex), dto.cumulativeDepositInterest, dto.cumulativeBorrowInterest);
    }
    constructor(indexedPosition, tokenIndex, inUseCount, previousIndex, cumulativeDepositInterest, cumulativeBorrowInterest) {
        this.indexedPosition = indexedPosition;
        this.tokenIndex = tokenIndex;
        this.inUseCount = inUseCount;
        this.previousIndex = previousIndex;
        this.cumulativeDepositInterest = cumulativeDepositInterest;
        this.cumulativeBorrowInterest = cumulativeBorrowInterest;
    }
    isActive() {
        return this.tokenIndex !== TokenPosition.TokenIndexUnset;
    }
    /**
     *
     * @param bank
     * @returns native balance
     */
    balance(bank) {
        if (this.indexedPosition.isPos()) {
            return bank.depositIndex.mul(this.indexedPosition);
        }
        else {
            return bank.borrowIndex.mul(this.indexedPosition);
        }
    }
    /**
     *
     * @param bank
     * @returns native deposits, 0 if position has borrows
     */
    deposits(bank) {
        if (this.indexedPosition && this.indexedPosition.lt(ZERO_I80F48())) {
            return ZERO_I80F48();
        }
        return this.balance(bank);
    }
    /**
     *
     * @param bank
     * @returns native borrows, 0 if position has deposits
     */
    borrows(bank) {
        if (this.indexedPosition && this.indexedPosition.gt(ZERO_I80F48())) {
            return ZERO_I80F48();
        }
        return this.balance(bank).abs();
    }
    /**
     * @param bank
     * @returns UI balance, is signed
     */
    balanceUi(bank) {
        return toUiDecimals(this.balance(bank), bank.mintDecimals);
    }
    /**
     * @param bank
     * @returns UI deposits, 0 if position has borrows
     */
    depositsUi(bank) {
        return toUiDecimals(this.deposits(bank), bank.mintDecimals);
    }
    /**
     * @param bank
     * @returns UI borrows, 0 if position has deposits
     */
    borrowsUi(bank) {
        return toUiDecimals(this.borrows(bank), bank.mintDecimals);
    }
    toString(group, index) {
        let extra = '';
        if (group) {
            const bank = group.getFirstBankByTokenIndex(this.tokenIndex);
            if (bank) {
                const native = this.balance(bank);
                extra += ', native: ' + native.toNumber();
                extra += ', ui: ' + this.balanceUi(bank);
                extra += ', tokenName: ' + bank.name;
            }
        }
        return ((index !== undefined ? 'index: ' + index : '') +
            ', tokenIndex: ' +
            this.tokenIndex +
            ', inUseCount: ' +
            this.inUseCount +
            ', indexedValue: ' +
            this.indexedPosition.toNumber() +
            extra);
    }
}
export class TokenPositionDto {
    indexedPosition;
    tokenIndex;
    inUseCount;
    reserved;
    previousIndex;
    cumulativeDepositInterest;
    cumulativeBorrowInterest;
    constructor(indexedPosition, tokenIndex, inUseCount, reserved, previousIndex, cumulativeDepositInterest, cumulativeBorrowInterest) {
        this.indexedPosition = indexedPosition;
        this.tokenIndex = tokenIndex;
        this.inUseCount = inUseCount;
        this.reserved = reserved;
        this.previousIndex = previousIndex;
        this.cumulativeDepositInterest = cumulativeDepositInterest;
        this.cumulativeBorrowInterest = cumulativeBorrowInterest;
    }
}
export class Serum3Orders {
    openOrders;
    marketIndex;
    baseTokenIndex;
    quoteTokenIndex;
    static Serum3MarketIndexUnset = 65535;
    static from(dto) {
        return new Serum3Orders(dto.openOrders, dto.marketIndex, dto.baseTokenIndex, dto.quoteTokenIndex);
    }
    constructor(openOrders, marketIndex, baseTokenIndex, quoteTokenIndex) {
        this.openOrders = openOrders;
        this.marketIndex = marketIndex;
        this.baseTokenIndex = baseTokenIndex;
        this.quoteTokenIndex = quoteTokenIndex;
    }
    isActive() {
        return this.marketIndex !== Serum3Orders.Serum3MarketIndexUnset;
    }
}
export class Serum3PositionDto {
    openOrders;
    marketIndex;
    baseBorrowsWithoutFee;
    quoteBorrowsWithoutFee;
    baseTokenIndex;
    quoteTokenIndex;
    reserved;
    constructor(openOrders, marketIndex, baseBorrowsWithoutFee, quoteBorrowsWithoutFee, baseTokenIndex, quoteTokenIndex, reserved) {
        this.openOrders = openOrders;
        this.marketIndex = marketIndex;
        this.baseBorrowsWithoutFee = baseBorrowsWithoutFee;
        this.quoteBorrowsWithoutFee = quoteBorrowsWithoutFee;
        this.baseTokenIndex = baseTokenIndex;
        this.quoteTokenIndex = quoteTokenIndex;
        this.reserved = reserved;
    }
}
export class PerpPosition {
    marketIndex;
    settlePnlLimitWindow;
    settlePnlLimitSettledInCurrentWindowNative;
    basePositionLots;
    quotePositionNative;
    quoteRunningNative;
    longSettledFunding;
    shortSettledFunding;
    bidsBaseLots;
    asksBaseLots;
    takerBaseLots;
    takerQuoteLots;
    cumulativeLongFunding;
    cumulativeShortFunding;
    makerVolume;
    takerVolume;
    perpSpotTransfers;
    avgEntryPricePerBaseLot;
    realizedTradePnlNative;
    realizedOtherPnlNative;
    settlePnlLimitRealizedTrade;
    realizedPnlForPositionNative;
    static PerpMarketIndexUnset = 65535;
    static from(dto) {
        return new PerpPosition(dto.marketIndex, dto.settlePnlLimitWindow, dto.settlePnlLimitSettledInCurrentWindowNative, dto.basePositionLots, I80F48.from(dto.quotePositionNative), dto.quoteRunningNative, I80F48.from(dto.longSettledFunding), I80F48.from(dto.shortSettledFunding), dto.bidsBaseLots, dto.asksBaseLots, dto.takerBaseLots, dto.takerQuoteLots, dto.cumulativeLongFunding, dto.cumulativeShortFunding, dto.makerVolume, dto.takerVolume, dto.perpSpotTransfers, dto.avgEntryPricePerBaseLot, I80F48.from(dto.realizedTradePnlNative), I80F48.from(dto.realizedOtherPnlNative), dto.settlePnlLimitRealizedTrade, I80F48.from(dto.realizedPnlForPositionNative));
    }
    static emptyFromPerpMarketIndex(perpMarketIndex) {
        return new PerpPosition(perpMarketIndex, 0, new BN(0), new BN(0), ZERO_I80F48(), new BN(0), ZERO_I80F48(), ZERO_I80F48(), new BN(0), new BN(0), new BN(0), new BN(0), 0, 0, new BN(0), new BN(0), new BN(0), 0, ZERO_I80F48(), ZERO_I80F48(), new BN(0), ZERO_I80F48());
    }
    constructor(marketIndex, settlePnlLimitWindow, settlePnlLimitSettledInCurrentWindowNative, basePositionLots, quotePositionNative, quoteRunningNative, longSettledFunding, shortSettledFunding, bidsBaseLots, asksBaseLots, takerBaseLots, takerQuoteLots, cumulativeLongFunding, cumulativeShortFunding, makerVolume, takerVolume, perpSpotTransfers, avgEntryPricePerBaseLot, realizedTradePnlNative, realizedOtherPnlNative, settlePnlLimitRealizedTrade, realizedPnlForPositionNative) {
        this.marketIndex = marketIndex;
        this.settlePnlLimitWindow = settlePnlLimitWindow;
        this.settlePnlLimitSettledInCurrentWindowNative = settlePnlLimitSettledInCurrentWindowNative;
        this.basePositionLots = basePositionLots;
        this.quotePositionNative = quotePositionNative;
        this.quoteRunningNative = quoteRunningNative;
        this.longSettledFunding = longSettledFunding;
        this.shortSettledFunding = shortSettledFunding;
        this.bidsBaseLots = bidsBaseLots;
        this.asksBaseLots = asksBaseLots;
        this.takerBaseLots = takerBaseLots;
        this.takerQuoteLots = takerQuoteLots;
        this.cumulativeLongFunding = cumulativeLongFunding;
        this.cumulativeShortFunding = cumulativeShortFunding;
        this.makerVolume = makerVolume;
        this.takerVolume = takerVolume;
        this.perpSpotTransfers = perpSpotTransfers;
        this.avgEntryPricePerBaseLot = avgEntryPricePerBaseLot;
        this.realizedTradePnlNative = realizedTradePnlNative;
        this.realizedOtherPnlNative = realizedOtherPnlNative;
        this.settlePnlLimitRealizedTrade = settlePnlLimitRealizedTrade;
        this.realizedPnlForPositionNative = realizedPnlForPositionNative;
    }
    isActive() {
        return this.marketIndex !== PerpPosition.PerpMarketIndexUnset;
    }
    getBasePositionNative(perpMarket) {
        return I80F48.fromI64(this.basePositionLots.mul(perpMarket.baseLotSize));
    }
    getBasePositionUi(perpMarket, useEventQueue) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        return perpMarket.baseLotsToUi(useEventQueue
            ? this.basePositionLots.add(this.takerBaseLots)
            : this.basePositionLots);
    }
    getQuotePositionUi(perpMarket, useEventQueue) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        const quotePositionUi = toUiDecimalsForQuote(this.quotePositionNative);
        return useEventQueue
            ? quotePositionUi + perpMarket.quoteLotsToUi(this.takerQuoteLots)
            : quotePositionUi;
    }
    getNotionalValueUi(perpMarket, useEventQueue) {
        return (this.getBasePositionUi(perpMarket, useEventQueue) * perpMarket.uiPrice);
    }
    getUnsettledFunding(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        if (this.basePositionLots.gt(new BN(0))) {
            return perpMarket.longFunding
                .sub(this.longSettledFunding)
                .mul(I80F48.fromI64(this.basePositionLots));
        }
        else if (this.basePositionLots.lt(new BN(0))) {
            return perpMarket.shortFunding
                .sub(this.shortSettledFunding)
                .mul(I80F48.fromI64(this.basePositionLots));
        }
        return ZERO_I80F48();
    }
    getEquityUi(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        return toUiDecimalsForQuote(this.getEquity(perpMarket));
    }
    getEquity(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        const lotsToQuote = I80F48.fromI64(perpMarket.baseLotSize).mul(perpMarket.price);
        const baseLots = I80F48.fromI64(this.basePositionLots.add(this.takerBaseLots));
        const unsettledFunding = this.getUnsettledFunding(perpMarket);
        const takerQuote = I80F48.fromI64(new BN(this.takerQuoteLots).mul(perpMarket.quoteLotSize));
        const quoteCurrent = this.quotePositionNative
            .sub(unsettledFunding)
            .add(takerQuote);
        return baseLots.mul(lotsToQuote).add(quoteCurrent);
    }
    hasOpenOrders() {
        const zero = new BN(0);
        return (!this.asksBaseLots.eq(zero) ||
            !this.bidsBaseLots.eq(zero) ||
            !this.takerBaseLots.eq(zero) ||
            !this.takerQuoteLots.eq(zero));
    }
    getAverageEntryPrice(perpMarket) {
        return I80F48.fromNumber(this.avgEntryPricePerBaseLot).div(I80F48.fromI64(perpMarket.baseLotSize));
    }
    getAverageEntryPriceUi(perpMarket) {
        return perpMarket.priceNativeToUi(this.getAverageEntryPrice(perpMarket).toNumber());
    }
    getBreakEvenPriceUi(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        if (this.basePositionLots.eq(new BN(0))) {
            return 0;
        }
        return perpMarket.priceNativeToUi(-this.quoteRunningNative.toNumber() /
            this.basePositionLots.mul(perpMarket.baseLotSize).toNumber());
    }
    cumulativePnlOverPositionLifetimeUi(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        const priceChange = perpMarket.price.sub(this.getAverageEntryPrice(perpMarket));
        return toUiDecimalsForQuote(this.realizedPnlForPositionNative.add(this.getBasePositionNative(perpMarket).mul(priceChange)));
    }
    getUnsettledPnl(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        return this.quotePositionNative.add(this.getBasePositionNative(perpMarket).mul(perpMarket.price));
    }
    getUnsettledPnlUi(perpMarket) {
        return toUiDecimalsForQuote(this.getUnsettledPnl(perpMarket));
    }
    updateSettleLimit(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        const windowSize = perpMarket.settlePnlLimitWindowSizeTs;
        const windowStart = new BN(this.settlePnlLimitWindow).mul(windowSize);
        const windowEnd = windowStart.add(windowSize);
        const nowTs = new BN(Date.now() / 1000);
        const newWindow = nowTs.gte(windowEnd) || nowTs.lt(windowStart);
        if (newWindow) {
            this.settlePnlLimitWindow = nowTs.div(windowSize).toNumber();
            this.settlePnlLimitSettledInCurrentWindowNative = new BN(0);
        }
    }
    availableSettleLimit(perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        if (perpMarket.settlePnlLimitFactor < 0) {
            return [RUST_I64_MIN(), RUST_I64_MAX()];
        }
        const baseNative = I80F48.fromI64(this.basePositionLots.mul(perpMarket.baseLotSize));
        const positionValue = I80F48.fromNumber(perpMarket.stablePriceModel.stablePrice)
            .mul(baseNative)
            .toNumber();
        const unrealized = new BN(perpMarket.settlePnlLimitFactor * positionValue);
        const used = new BN(this.settlePnlLimitSettledInCurrentWindowNative.toNumber());
        let minPnl = unrealized.neg().sub(used);
        let maxPnl = unrealized.sub(used);
        const realizedTrade = this.settlePnlLimitRealizedTrade;
        if (realizedTrade.gte(new BN(0))) {
            maxPnl = maxPnl.add(realizedTrade);
        }
        else {
            minPnl = minPnl.add(realizedTrade);
        }
        const realizedOther = new BN(this.realizedOtherPnlNative.toNumber());
        if (realizedOther.gte(new BN(0))) {
            maxPnl = maxPnl.add(realizedOther);
        }
        else {
            minPnl = minPnl.add(realizedOther);
        }
        return [BN.min(minPnl, new BN(0)), BN.max(maxPnl, new BN(0))];
    }
    applyPnlSettleLimit(pnl, perpMarket) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        if (perpMarket.settlePnlLimitFactor < 0) {
            return pnl;
        }
        const [minPnl, maxPnl] = this.availableSettleLimit(perpMarket);
        if (pnl.lt(ZERO_I80F48())) {
            return pnl.max(I80F48.fromI64(minPnl));
        }
        else {
            return pnl.min(I80F48.fromI64(maxPnl));
        }
    }
    getSettleablePnl(group, perpMarket, account) {
        if (perpMarket.perpMarketIndex !== this.marketIndex) {
            throw new Error("PerpPosition doesn't belong to the given market!");
        }
        this.updateSettleLimit(perpMarket);
        const perpSettleHealth = account.getPerpSettleHealth(group);
        const limitedUnsettled = this.applyPnlSettleLimit(this.getUnsettledPnl(perpMarket), perpMarket);
        if (limitedUnsettled.lt(ZERO_I80F48())) {
            return limitedUnsettled.max(perpSettleHealth.max(ZERO_I80F48()).neg());
        }
        return limitedUnsettled;
    }
    getSettleablePnlUi(group, perpMarket, account) {
        return toUiDecimalsForQuote(this.getSettleablePnl(group, perpMarket, account));
    }
    canSettlePnl(group, perpMarket, account) {
        return !this.getSettleablePnl(group, perpMarket, account).eq(ZERO_I80F48());
    }
    toString(perpMarket) {
        return perpMarket
            ? 'market - ' +
                perpMarket.name +
                ', basePositionLots - ' +
                perpMarket.baseLotsToUi(this.basePositionLots) +
                ', quotePositive - ' +
                toUiDecimalsForQuote(this.quotePositionNative.toNumber()) +
                ', bidsBaseLots - ' +
                perpMarket.baseLotsToUi(this.bidsBaseLots) +
                ', asksBaseLots - ' +
                perpMarket.baseLotsToUi(this.asksBaseLots) +
                ', takerBaseLots - ' +
                perpMarket.baseLotsToUi(this.takerBaseLots) +
                ', takerQuoteLots - ' +
                perpMarket.quoteLotsToUi(this.takerQuoteLots) +
                ', unsettled pnl - ' +
                this.getUnsettledPnlUi(perpMarket).toString()
            : '';
    }
}
export class PerpPositionDto {
    marketIndex;
    settlePnlLimitWindow;
    settlePnlLimitSettledInCurrentWindowNative;
    basePositionLots;
    quotePositionNative;
    quoteRunningNative;
    longSettledFunding;
    shortSettledFunding;
    bidsBaseLots;
    asksBaseLots;
    takerBaseLots;
    takerQuoteLots;
    cumulativeLongFunding;
    cumulativeShortFunding;
    makerVolume;
    takerVolume;
    perpSpotTransfers;
    avgEntryPricePerBaseLot;
    realizedTradePnlNative;
    realizedOtherPnlNative;
    settlePnlLimitRealizedTrade;
    realizedPnlForPositionNative;
    constructor(marketIndex, settlePnlLimitWindow, settlePnlLimitSettledInCurrentWindowNative, basePositionLots, quotePositionNative, quoteRunningNative, longSettledFunding, shortSettledFunding, bidsBaseLots, asksBaseLots, takerBaseLots, takerQuoteLots, cumulativeLongFunding, cumulativeShortFunding, makerVolume, takerVolume, perpSpotTransfers, avgEntryPricePerBaseLot, realizedTradePnlNative, realizedOtherPnlNative, settlePnlLimitRealizedTrade, realizedPnlForPositionNative) {
        this.marketIndex = marketIndex;
        this.settlePnlLimitWindow = settlePnlLimitWindow;
        this.settlePnlLimitSettledInCurrentWindowNative = settlePnlLimitSettledInCurrentWindowNative;
        this.basePositionLots = basePositionLots;
        this.quotePositionNative = quotePositionNative;
        this.quoteRunningNative = quoteRunningNative;
        this.longSettledFunding = longSettledFunding;
        this.shortSettledFunding = shortSettledFunding;
        this.bidsBaseLots = bidsBaseLots;
        this.asksBaseLots = asksBaseLots;
        this.takerBaseLots = takerBaseLots;
        this.takerQuoteLots = takerQuoteLots;
        this.cumulativeLongFunding = cumulativeLongFunding;
        this.cumulativeShortFunding = cumulativeShortFunding;
        this.makerVolume = makerVolume;
        this.takerVolume = takerVolume;
        this.perpSpotTransfers = perpSpotTransfers;
        this.avgEntryPricePerBaseLot = avgEntryPricePerBaseLot;
        this.realizedTradePnlNative = realizedTradePnlNative;
        this.realizedOtherPnlNative = realizedOtherPnlNative;
        this.settlePnlLimitRealizedTrade = settlePnlLimitRealizedTrade;
        this.realizedPnlForPositionNative = realizedPnlForPositionNative;
    }
}
export class PerpOo {
    sideAndTree;
    orderMarket;
    clientId;
    id;
    static OrderMarketUnset = 65535;
    static from(dto) {
        return new PerpOo(dto.sideAndTree, dto.market, dto.clientId, dto.id);
    }
    constructor(sideAndTree, orderMarket, clientId, id) {
        this.sideAndTree = sideAndTree;
        this.orderMarket = orderMarket;
        this.clientId = clientId;
        this.id = id;
    }
    isActive() {
        return this.orderMarket !== PerpOo.OrderMarketUnset;
    }
}
export class PerpOoDto {
    sideAndTree;
    market;
    clientId;
    id;
    constructor(sideAndTree, market, clientId, id) {
        this.sideAndTree = sideAndTree;
        this.market = market;
        this.clientId = clientId;
        this.id = id;
    }
}
export class HealthType {
    static maint = { maint: {} };
    static init = { init: {} };
    static liquidationEnd = { liquidationEnd: {} };
}
