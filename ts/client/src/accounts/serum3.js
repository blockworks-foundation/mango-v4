import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { OPENBOOK_PROGRAM_ID } from '../constants';
import { MAX_I80F48, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
export class Serum3Market {
    publicKey;
    group;
    baseTokenIndex;
    quoteTokenIndex;
    serumProgram;
    serumMarketExternal;
    marketIndex;
    registrationTime;
    reduceOnly;
    name;
    static from(publicKey, obj) {
        return new Serum3Market(publicKey, obj.group, obj.baseTokenIndex, obj.quoteTokenIndex, obj.name, obj.serumProgram, obj.serumMarketExternal, obj.marketIndex, obj.registrationTime, obj.reduceOnly == 1);
    }
    constructor(publicKey, group, baseTokenIndex, quoteTokenIndex, name, serumProgram, serumMarketExternal, marketIndex, registrationTime, reduceOnly) {
        this.publicKey = publicKey;
        this.group = group;
        this.baseTokenIndex = baseTokenIndex;
        this.quoteTokenIndex = quoteTokenIndex;
        this.serumProgram = serumProgram;
        this.serumMarketExternal = serumMarketExternal;
        this.marketIndex = marketIndex;
        this.registrationTime = registrationTime;
        this.reduceOnly = reduceOnly;
        this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    }
    async findOoPda(programId, mangoAccount) {
        const [openOrderPublicKey] = await PublicKey.findProgramAddress([
            Buffer.from('Serum3OO'),
            mangoAccount.toBuffer(),
            this.publicKey.toBuffer(),
        ], programId);
        return openOrderPublicKey;
    }
    getFeeRates(taker = true) {
        // See https://github.com/openbook-dex/program/blob/master/dex/src/fees.rs#L81
        const ratesBps = this.name === 'USDT/USDC'
            ? { maker: -0.5, taker: 1 }
            : { maker: -2, taker: 4 };
        return taker ? ratesBps.maker * 0.0001 : ratesBps.taker * 0.0001;
    }
    /**
     *
     * @param group
     * @returns maximum leverage one can bid on this market, this is only for display purposes,
     *  also see getMaxQuoteForSerum3BidUi and getMaxBaseForSerum3AskUi
     */
    maxBidLeverage(group) {
        const baseBank = group.getFirstBankByTokenIndex(this.baseTokenIndex);
        const quoteBank = group.getFirstBankByTokenIndex(this.quoteTokenIndex);
        if (quoteBank.initLiabWeight.sub(baseBank.initAssetWeight).lte(ZERO_I80F48())) {
            return MAX_I80F48().toNumber();
        }
        return ONE_I80F48()
            .div(quoteBank.initLiabWeight.sub(baseBank.initAssetWeight))
            .toNumber();
    }
    /**
     *
     * @param group
     * @returns maximum leverage one can ask on this market, this is only for display purposes,
     *  also see getMaxQuoteForSerum3BidUi and getMaxBaseForSerum3AskUi
     */
    maxAskLeverage(group) {
        const baseBank = group.getFirstBankByTokenIndex(this.baseTokenIndex);
        const quoteBank = group.getFirstBankByTokenIndex(this.quoteTokenIndex);
        if (baseBank.initLiabWeight.sub(quoteBank.initAssetWeight).lte(ZERO_I80F48())) {
            return MAX_I80F48().toNumber();
        }
        return ONE_I80F48()
            .div(baseBank.initLiabWeight.sub(quoteBank.initAssetWeight))
            .toNumber();
    }
    async loadBids(client, group) {
        const serum3MarketExternal = group.getSerum3ExternalMarket(this.serumMarketExternal);
        return await serum3MarketExternal.loadBids(client.program.provider.connection);
    }
    async loadAsks(client, group) {
        const serum3MarketExternal = group.getSerum3ExternalMarket(this.serumMarketExternal);
        return await serum3MarketExternal.loadAsks(client.program.provider.connection);
    }
    async logOb(client, group) {
        let res = ``;
        res += `  ${this.name} OrderBook`;
        let orders = await this?.loadAsks(client, group);
        for (const order of orders.items(true)) {
            res += `\n  ${order.price.toString().padStart(10)}, ${order.size
                .toString()
                .padStart(10)}`;
        }
        res += `\n  --------------------------`;
        orders = await this?.loadBids(client, group);
        for (const order of orders.items(true)) {
            res += `\n  ${order.price.toString().padStart(10)}, ${order.size
                .toString()
                .padStart(10)}`;
        }
        return res;
    }
}
export class Serum3SelfTradeBehavior {
    static decrementTake = { decrementTake: {} };
    static cancelProvide = { cancelProvide: {} };
    static abortTransaction = { abortTransaction: {} };
}
export class Serum3OrderType {
    static limit = { limit: {} };
    static immediateOrCancel = { immediateOrCancel: {} };
    static postOnly = { postOnly: {} };
}
export class Serum3Side {
    static bid = { bid: {} };
    static ask = { ask: {} };
}
export async function generateSerum3MarketExternalVaultSignerAddress(cluster, serum3Market, serum3MarketExternal) {
    return await PublicKey.createProgramAddress([
        serum3Market.serumMarketExternal.toBuffer(),
        serum3MarketExternal.decoded.vaultSignerNonce.toArrayLike(Buffer, 'le', 8),
    ], OPENBOOK_PROGRAM_ID[cluster]);
}
