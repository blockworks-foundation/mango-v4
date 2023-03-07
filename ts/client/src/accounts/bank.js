import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { toUiDecimals } from '../utils';
export class Bank {
    publicKey;
    group;
    mint;
    vault;
    oracle;
    stablePriceModel;
    indexLastUpdated;
    bankRateLastUpdated;
    tokenIndex;
    mintDecimals;
    bankNum;
    minVaultToDepositsRatio;
    netBorrowLimitWindowSizeTs;
    lastNetBorrowsWindowStartTs;
    netBorrowLimitPerWindowQuote;
    netBorrowsInWindow;
    borrowWeightScaleStartQuote;
    depositWeightScaleStartQuote;
    reduceOnly;
    name;
    oracleConfig;
    depositIndex;
    borrowIndex;
    indexedDeposits;
    indexedBorrows;
    avgUtilization;
    adjustmentFactor;
    maxRate;
    rate0;
    rate1;
    util0;
    util1;
    _price;
    _uiPrice;
    _oracleLastUpdatedSlot;
    collectedFeesNative;
    loanFeeRate;
    loanOriginationFeeRate;
    initAssetWeight;
    maintAssetWeight;
    initLiabWeight;
    maintLiabWeight;
    liquidationFee;
    dust;
    static from(publicKey, obj) {
        return new Bank(publicKey, obj.group, obj.name, obj.mint, obj.vault, obj.oracle, obj.oracleConfig, obj.stablePriceModel, obj.depositIndex, obj.borrowIndex, obj.indexedDeposits, obj.indexedBorrows, obj.indexLastUpdated, obj.bankRateLastUpdated, obj.avgUtilization, obj.adjustmentFactor, obj.util0, obj.rate0, obj.util1, obj.rate1, obj.maxRate, obj.collectedFeesNative, obj.loanOriginationFeeRate, obj.loanFeeRate, obj.maintAssetWeight, obj.initAssetWeight, obj.maintLiabWeight, obj.initLiabWeight, obj.liquidationFee, obj.dust, obj.flashLoanTokenAccountInitial, obj.flashLoanApprovedAmount, obj.tokenIndex, obj.mintDecimals, obj.bankNum, obj.minVaultToDepositsRatio, obj.netBorrowLimitWindowSizeTs, obj.lastNetBorrowsWindowStartTs, obj.netBorrowLimitPerWindowQuote, obj.netBorrowsInWindow, obj.borrowWeightScaleStartQuote, obj.depositWeightScaleStartQuote, obj.reduceOnly == 1);
    }
    constructor(publicKey, group, name, mint, vault, oracle, oracleConfig, stablePriceModel, depositIndex, borrowIndex, indexedDeposits, indexedBorrows, indexLastUpdated, bankRateLastUpdated, avgUtilization, adjustmentFactor, util0, rate0, util1, rate1, maxRate, collectedFeesNative, loanOriginationFeeRate, loanFeeRate, maintAssetWeight, initAssetWeight, maintLiabWeight, initLiabWeight, liquidationFee, dust, flashLoanTokenAccountInitial, flashLoanApprovedAmount, tokenIndex, mintDecimals, bankNum, minVaultToDepositsRatio, netBorrowLimitWindowSizeTs, lastNetBorrowsWindowStartTs, netBorrowLimitPerWindowQuote, netBorrowsInWindow, borrowWeightScaleStartQuote, depositWeightScaleStartQuote, reduceOnly) {
        this.publicKey = publicKey;
        this.group = group;
        this.mint = mint;
        this.vault = vault;
        this.oracle = oracle;
        this.stablePriceModel = stablePriceModel;
        this.indexLastUpdated = indexLastUpdated;
        this.bankRateLastUpdated = bankRateLastUpdated;
        this.tokenIndex = tokenIndex;
        this.mintDecimals = mintDecimals;
        this.bankNum = bankNum;
        this.minVaultToDepositsRatio = minVaultToDepositsRatio;
        this.netBorrowLimitWindowSizeTs = netBorrowLimitWindowSizeTs;
        this.lastNetBorrowsWindowStartTs = lastNetBorrowsWindowStartTs;
        this.netBorrowLimitPerWindowQuote = netBorrowLimitPerWindowQuote;
        this.netBorrowsInWindow = netBorrowsInWindow;
        this.borrowWeightScaleStartQuote = borrowWeightScaleStartQuote;
        this.depositWeightScaleStartQuote = depositWeightScaleStartQuote;
        this.reduceOnly = reduceOnly;
        this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
        this.oracleConfig = {
            confFilter: I80F48.from(oracleConfig.confFilter),
            maxStalenessSlots: oracleConfig.maxStalenessSlots,
        };
        this.depositIndex = I80F48.from(depositIndex);
        this.borrowIndex = I80F48.from(borrowIndex);
        this.indexedDeposits = I80F48.from(indexedDeposits);
        this.indexedBorrows = I80F48.from(indexedBorrows);
        this.avgUtilization = I80F48.from(avgUtilization);
        this.adjustmentFactor = I80F48.from(adjustmentFactor);
        this.maxRate = I80F48.from(maxRate);
        this.util0 = I80F48.from(util0);
        this.rate0 = I80F48.from(rate0);
        this.util1 = I80F48.from(util1);
        this.rate1 = I80F48.from(rate1);
        this.collectedFeesNative = I80F48.from(collectedFeesNative);
        this.loanFeeRate = I80F48.from(loanFeeRate);
        this.loanOriginationFeeRate = I80F48.from(loanOriginationFeeRate);
        this.maintAssetWeight = I80F48.from(maintAssetWeight);
        this.initAssetWeight = I80F48.from(initAssetWeight);
        this.maintLiabWeight = I80F48.from(maintLiabWeight);
        this.initLiabWeight = I80F48.from(initLiabWeight);
        this.liquidationFee = I80F48.from(liquidationFee);
        this.dust = I80F48.from(dust);
        this._price = undefined;
        this._uiPrice = undefined;
        this._oracleLastUpdatedSlot = undefined;
    }
    toString() {
        return ('Bank ' +
            '\n public key - ' +
            this.publicKey.toBase58() +
            '\n token index - ' +
            this.tokenIndex +
            '\n token name - ' +
            this.name +
            '\n vault - ' +
            this.vault.toBase58() +
            '\n mintDecimals - ' +
            this.mintDecimals +
            '\n oracle - ' +
            this.oracle.toBase58() +
            '\n price - ' +
            this._price?.toString() +
            '\n uiPrice - ' +
            this._uiPrice +
            '\n deposit index - ' +
            this.depositIndex.toString() +
            '\n borrow index - ' +
            this.borrowIndex.toString() +
            '\n indexedDeposits - ' +
            this.indexedDeposits.toString() +
            '\n indexedBorrows - ' +
            this.indexedBorrows.toString() +
            '\n indexLastUpdated - ' +
            new Date(this.indexLastUpdated.toNumber() * 1000) +
            '\n bankRateLastUpdated - ' +
            new Date(this.bankRateLastUpdated.toNumber() * 1000) +
            '\n avgUtilization - ' +
            this.avgUtilization.toString() +
            '\n adjustmentFactor - ' +
            this.adjustmentFactor.toString() +
            '\n maxRate - ' +
            this.maxRate.toString() +
            '\n util0 - ' +
            this.util0.toString() +
            '\n rate0 - ' +
            this.rate0.toString() +
            '\n util1 - ' +
            this.util1.toString() +
            '\n rate1 - ' +
            this.rate1.toString() +
            '\n loanFeeRate - ' +
            this.loanFeeRate.toString() +
            '\n loanOriginationFeeRate - ' +
            this.loanOriginationFeeRate.toString() +
            '\n maintAssetWeight - ' +
            this.maintAssetWeight.toString() +
            '\n initAssetWeight - ' +
            this.initAssetWeight.toString() +
            '\n maintLiabWeight - ' +
            this.maintLiabWeight.toString() +
            '\n initLiabWeight - ' +
            this.initLiabWeight.toString() +
            '\n liquidationFee - ' +
            this.liquidationFee.toString() +
            '\n uiDeposits() - ' +
            this.uiDeposits() +
            '\n uiBorrows() - ' +
            this.uiBorrows() +
            '\n getDepositRate() - ' +
            this.getDepositRate().toString() +
            '\n getBorrowRate() - ' +
            this.getBorrowRate().toString());
    }
    scaledInitAssetWeight(price) {
        const depositsQuote = this.nativeDeposits().mul(price);
        if (this.depositWeightScaleStartQuote >= Number.MAX_SAFE_INTEGER ||
            depositsQuote.lte(I80F48.fromNumber(this.depositWeightScaleStartQuote))) {
            return this.initAssetWeight;
        }
        return this.initAssetWeight.mul(I80F48.fromNumber(this.depositWeightScaleStartQuote).div(depositsQuote));
    }
    scaledInitLiabWeight(price) {
        const borrowsQuote = this.nativeBorrows().mul(price);
        if (this.borrowWeightScaleStartQuote >= Number.MAX_SAFE_INTEGER ||
            borrowsQuote.lte(I80F48.fromNumber(this.borrowWeightScaleStartQuote))) {
            return this.initLiabWeight;
        }
        return this.initLiabWeight.mul(borrowsQuote.div(I80F48.fromNumber(this.borrowWeightScaleStartQuote)));
    }
    get price() {
        if (!this._price) {
            throw new Error(`Undefined price for bank ${this.publicKey} with tokenIndex ${this.tokenIndex}!`);
        }
        return this._price;
    }
    get uiPrice() {
        if (!this._uiPrice) {
            throw new Error(`Undefined uiPrice for bank ${this.publicKey} with tokenIndex ${this.tokenIndex}!`);
        }
        return this._uiPrice;
    }
    get oracleLastUpdatedSlot() {
        if (!this._oracleLastUpdatedSlot) {
            throw new Error(`Undefined oracleLastUpdatedSlot for bank ${this.publicKey} with tokenIndex ${this.tokenIndex}!`);
        }
        return this._oracleLastUpdatedSlot;
    }
    nativeDeposits() {
        return this.indexedDeposits.mul(this.depositIndex);
    }
    nativeBorrows() {
        return this.indexedBorrows.mul(this.borrowIndex);
    }
    uiDeposits() {
        return toUiDecimals(this.indexedDeposits.mul(this.depositIndex), this.mintDecimals);
    }
    uiBorrows() {
        return toUiDecimals(this.indexedBorrows.mul(this.borrowIndex), this.mintDecimals);
    }
    /**
     *
     * @returns borrow rate, 0 is 0% where 1 is 100%
     */
    getBorrowRate() {
        const totalBorrows = this.nativeBorrows();
        const totalDeposits = this.nativeDeposits();
        if (totalDeposits.isZero() && totalBorrows.isZero()) {
            return ZERO_I80F48();
        }
        if (totalDeposits.lte(totalBorrows)) {
            return this.maxRate;
        }
        const utilization = totalBorrows.div(totalDeposits);
        if (utilization.gt(this.util1)) {
            const extraUtil = utilization.sub(this.util1);
            const slope = this.maxRate
                .sub(this.rate1)
                .div(I80F48.fromNumber(1).sub(this.util1));
            return this.rate1.add(slope.mul(extraUtil));
        }
        else if (utilization.gt(this.util0)) {
            const extraUtil = utilization.sub(this.util0);
            const slope = this.maxRate
                .sub(this.rate0)
                .div(I80F48.fromNumber(1).sub(this.util0));
            return this.rate0.add(slope.mul(extraUtil));
        }
        else {
            const slope = this.rate0.div(this.util0);
            return slope.mul(utilization);
        }
    }
    /**
     *
     * @returns borrow rate percentage
     */
    getBorrowRateUi() {
        return this.getBorrowRate().toNumber() * 100;
    }
    /**
     *
     * @returns deposit rate, 0 is 0% where 1 is 100%
     */
    getDepositRate() {
        const borrowRate = this.getBorrowRate();
        const totalBorrows = this.nativeBorrows();
        const totalDeposits = this.nativeDeposits();
        if (totalDeposits.isZero() && totalBorrows.isZero()) {
            return ZERO_I80F48();
        }
        else if (totalDeposits.isZero()) {
            return this.maxRate;
        }
        const utilization = totalBorrows.div(totalDeposits);
        return utilization.mul(borrowRate);
    }
    /**
     *
     * @returns deposit rate percentage
     */
    getDepositRateUi() {
        return this.getDepositRate().toNumber() * 100;
    }
}
export class MintInfo {
    publicKey;
    group;
    tokenIndex;
    mint;
    banks;
    vaults;
    oracle;
    registrationTime;
    groupInsuranceFund;
    static from(publicKey, obj) {
        return new MintInfo(publicKey, obj.group, obj.tokenIndex, obj.mint, obj.banks, obj.vaults, obj.oracle, obj.registrationTime, obj.groupInsuranceFund == 1);
    }
    constructor(publicKey, group, tokenIndex, mint, banks, vaults, oracle, registrationTime, groupInsuranceFund) {
        this.publicKey = publicKey;
        this.group = group;
        this.tokenIndex = tokenIndex;
        this.mint = mint;
        this.banks = banks;
        this.vaults = vaults;
        this.oracle = oracle;
        this.registrationTime = registrationTime;
        this.groupInsuranceFund = groupInsuranceFund;
    }
    firstBank() {
        return this.banks[0];
    }
    firstVault() {
        return this.vaults[0];
    }
    toString() {
        const res = 'mint ' +
            this.mint.toBase58() +
            '\n oracle ' +
            this.oracle.toBase58() +
            '\n banks ' +
            this.banks
                .filter((pk) => pk.toBase58() !== PublicKey.default.toBase58())
                .toString() +
            '\n vaults ' +
            this.vaults
                .filter((pk) => pk.toBase58() !== PublicKey.default.toBase58())
                .toString();
        return res;
    }
}
