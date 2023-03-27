import { BN } from '@coral-xyz/anchor';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { I80F48, I80F48Dto, ZERO_I80F48 } from '../numbers/I80F48';
import { As, toUiDecimals } from '../utils';
import { OracleProvider } from './oracle';

export type TokenIndex = number & As<'token-index'>;

export type OracleConfigDto = {
  confFilter: I80F48Dto;
  maxStalenessSlots: BN;
};

export type OracleConfig = {
  confFilter: I80F48;
  maxStalenessSlots: BN;
};

export type StablePriceModel = {
  stablePrice: number;
  lastUpdateTimestamp: BN;
  delayPrices: number[];
  delayAccumulatorPrice: number;
  delayAccumulatorTime: number;
  delayIntervalSeconds: number;
  delayGrowthLimit: number;
  stableGrowthLimit: number;
  lastDelayIntervalIndex: number;
};

export interface BankForHealth {
  tokenIndex: TokenIndex;
  maintAssetWeight: I80F48;
  initAssetWeight: I80F48;
  maintLiabWeight: I80F48;
  initLiabWeight: I80F48;
  price: I80F48;
  stablePriceModel: StablePriceModel;

  scaledInitAssetWeight(price: I80F48): I80F48;
  scaledInitLiabWeight(price: I80F48): I80F48;
}

export class Bank implements BankForHealth {
  public name: string;
  public oracleConfig: OracleConfig;
  public depositIndex: I80F48;
  public borrowIndex: I80F48;
  public indexedDeposits: I80F48;
  public indexedBorrows: I80F48;
  public avgUtilization: I80F48;
  public adjustmentFactor: I80F48;
  public maxRate: I80F48;
  public rate0: I80F48;
  public rate1: I80F48;
  public util0: I80F48;
  public util1: I80F48;
  public _price: I80F48 | undefined;
  public _uiPrice: number | undefined;
  public _oracleLastUpdatedSlot: number | undefined;
  public _oracleProvider: OracleProvider | undefined;
  public collectedFeesNative: I80F48;
  public loanFeeRate: I80F48;
  public loanOriginationFeeRate: I80F48;
  public initAssetWeight: I80F48;
  public maintAssetWeight: I80F48;
  public initLiabWeight: I80F48;
  public maintLiabWeight: I80F48;
  public liquidationFee: I80F48;
  public dust: I80F48;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      name: number[];
      mint: PublicKey;
      vault: PublicKey;
      oracle: PublicKey;
      oracleConfig: OracleConfigDto;
      stablePriceModel: StablePriceModel;
      depositIndex: I80F48Dto;
      borrowIndex: I80F48Dto;
      indexedDeposits: I80F48Dto;
      indexedBorrows: I80F48Dto;
      indexLastUpdated: BN;
      bankRateLastUpdated: BN;
      avgUtilization: I80F48Dto;
      adjustmentFactor: I80F48Dto;
      util0: I80F48Dto;
      rate0: I80F48Dto;
      util1: I80F48Dto;
      rate1: I80F48Dto;
      maxRate: I80F48Dto;
      collectedFeesNative: I80F48Dto;
      loanOriginationFeeRate: I80F48Dto;
      loanFeeRate: I80F48Dto;
      maintAssetWeight: I80F48Dto;
      initAssetWeight: I80F48Dto;
      maintLiabWeight: I80F48Dto;
      initLiabWeight: I80F48Dto;
      liquidationFee: I80F48Dto;
      dust: I80F48Dto;
      flashLoanTokenAccountInitial: BN;
      flashLoanApprovedAmount: BN;
      tokenIndex: number;
      mintDecimals: number;
      bankNum: number;
      minVaultToDepositsRatio: number;
      netBorrowLimitWindowSizeTs: BN;
      lastNetBorrowsWindowStartTs: BN;
      netBorrowLimitPerWindowQuote: BN;
      netBorrowsInWindow: BN;
      borrowWeightScaleStartQuote: number;
      depositWeightScaleStartQuote: number;
      reduceOnly: number;
    },
  ): Bank {
    return new Bank(
      publicKey,
      obj.group,
      obj.name,
      obj.mint,
      obj.vault,
      obj.oracle,
      obj.oracleConfig,
      obj.stablePriceModel,
      obj.depositIndex,
      obj.borrowIndex,
      obj.indexedDeposits,
      obj.indexedBorrows,
      obj.indexLastUpdated,
      obj.bankRateLastUpdated,
      obj.avgUtilization,
      obj.adjustmentFactor,
      obj.util0,
      obj.rate0,
      obj.util1,
      obj.rate1,
      obj.maxRate,
      obj.collectedFeesNative,
      obj.loanOriginationFeeRate,
      obj.loanFeeRate,
      obj.maintAssetWeight,
      obj.initAssetWeight,
      obj.maintLiabWeight,
      obj.initLiabWeight,
      obj.liquidationFee,
      obj.dust,
      obj.flashLoanTokenAccountInitial,
      obj.flashLoanApprovedAmount,
      obj.tokenIndex as TokenIndex,
      obj.mintDecimals,
      obj.bankNum,
      obj.minVaultToDepositsRatio,
      obj.netBorrowLimitWindowSizeTs,
      obj.lastNetBorrowsWindowStartTs,
      obj.netBorrowLimitPerWindowQuote,
      obj.netBorrowsInWindow,
      obj.borrowWeightScaleStartQuote,
      obj.depositWeightScaleStartQuote,
      obj.reduceOnly == 1,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    name: number[],
    public mint: PublicKey,
    public vault: PublicKey,
    public oracle: PublicKey,
    oracleConfig: OracleConfigDto,
    public stablePriceModel: StablePriceModel,
    depositIndex: I80F48Dto,
    borrowIndex: I80F48Dto,
    indexedDeposits: I80F48Dto,
    indexedBorrows: I80F48Dto,
    public indexLastUpdated: BN,
    public bankRateLastUpdated: BN,
    avgUtilization: I80F48Dto,
    adjustmentFactor: I80F48Dto,
    util0: I80F48Dto,
    rate0: I80F48Dto,
    util1: I80F48Dto,
    rate1: I80F48Dto,
    maxRate: I80F48Dto,
    collectedFeesNative: I80F48Dto,
    loanOriginationFeeRate: I80F48Dto,
    loanFeeRate: I80F48Dto,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    liquidationFee: I80F48Dto,
    dust: I80F48Dto,
    flashLoanTokenAccountInitial: BN,
    flashLoanApprovedAmount: BN,
    public tokenIndex: TokenIndex,
    public mintDecimals: number,
    public bankNum: number,
    public minVaultToDepositsRatio: number,
    public netBorrowLimitWindowSizeTs: BN,
    public lastNetBorrowsWindowStartTs: BN,
    public netBorrowLimitPerWindowQuote: BN,
    public netBorrowsInWindow: BN,
    public borrowWeightScaleStartQuote: number,
    public depositWeightScaleStartQuote: number,
    public reduceOnly: boolean,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.oracleConfig = {
      confFilter: I80F48.from(oracleConfig.confFilter),
      maxStalenessSlots: oracleConfig.maxStalenessSlots,
    } as OracleConfig;
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
    this._oracleProvider = undefined;
  }

  toString(): string {
    return (
      'Bank ' +
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
      this.getBorrowRate().toString()
    );
  }

  scaledInitAssetWeight(price: I80F48): I80F48 {
    const depositsQuote = this.nativeDeposits().mul(price);
    if (
      this.depositWeightScaleStartQuote >= Number.MAX_SAFE_INTEGER ||
      depositsQuote.lte(I80F48.fromNumber(this.depositWeightScaleStartQuote))
    ) {
      return this.initAssetWeight;
    }
    return this.initAssetWeight.mul(
      I80F48.fromNumber(this.depositWeightScaleStartQuote).div(depositsQuote),
    );
  }

  scaledInitLiabWeight(price: I80F48): I80F48 {
    const borrowsQuote = this.nativeBorrows().mul(price);
    if (
      this.borrowWeightScaleStartQuote >= Number.MAX_SAFE_INTEGER ||
      borrowsQuote.lte(I80F48.fromNumber(this.borrowWeightScaleStartQuote))
    ) {
      return this.initLiabWeight;
    }
    return this.initLiabWeight.mul(
      borrowsQuote.div(I80F48.fromNumber(this.borrowWeightScaleStartQuote)),
    );
  }

  get price(): I80F48 {
    if (this._price === undefined) {
      throw new Error(
        `Undefined price for bank ${this.publicKey} with tokenIndex ${this.tokenIndex}!`,
      );
    }
    return this._price;
  }

  get uiPrice(): number {
    if (this._uiPrice === undefined) {
      throw new Error(
        `Undefined uiPrice for bank ${this.publicKey} with tokenIndex ${this.tokenIndex}!`,
      );
    }
    return this._uiPrice;
  }

  get oracleLastUpdatedSlot(): number {
    if (this._oracleLastUpdatedSlot === undefined) {
      throw new Error(
        `Undefined oracleLastUpdatedSlot for bank ${this.publicKey} with tokenIndex ${this.tokenIndex}!`,
      );
    }
    return this._oracleLastUpdatedSlot;
  }

  get oracleProvider(): OracleProvider {
    if (this._oracleProvider === undefined) {
      throw new Error(
        `Undefined oracleProvider for bank ${this.publicKey} with tokenIndex ${this.tokenIndex}!`,
      );
    }
    return this._oracleProvider;
  }

  nativeDeposits(): I80F48 {
    return this.indexedDeposits.mul(this.depositIndex);
  }

  nativeBorrows(): I80F48 {
    return this.indexedBorrows.mul(this.borrowIndex);
  }

  uiDeposits(): number {
    return toUiDecimals(
      this.indexedDeposits.mul(this.depositIndex),
      this.mintDecimals,
    );
  }

  uiBorrows(): number {
    return toUiDecimals(
      this.indexedBorrows.mul(this.borrowIndex),
      this.mintDecimals,
    );
  }

  /**
   *
   * @returns borrow rate, 0 is 0% where 1 is 100%
   */
  getBorrowRate(): I80F48 {
    const totalBorrows = this.nativeBorrows();
    const totalDeposits = this.nativeDeposits();

    if (totalDeposits.isZero() && totalBorrows.isZero()) {
      return ZERO_I80F48();
    }
    if (totalDeposits.lte(totalBorrows)) {
      return this.maxRate;
    }

    const utilization = totalBorrows.div(totalDeposits);
    if (utilization.lte(this.util0)) {
      const slope = this.rate0.div(this.util0);
      return slope.mul(utilization);
    } else if (utilization.lt(this.util1)) {
      const extraUtil = utilization.sub(this.util0);
      const slope = this.rate1.sub(this.rate0).div(this.util1.sub(this.util0));
      return this.rate0.add(slope.mul(extraUtil));
    } else {
      const extraUtil = utilization.sub(this.util1);
      const slope = this.maxRate
        .sub(this.rate1)
        .div(I80F48.fromNumber(1).sub(this.util1));
      return this.rate1.add(slope.mul(extraUtil));
    }
  }

  /**
   *
   * @returns borrow rate percentage
   */
  getBorrowRateUi(): number {
    return this.getBorrowRate().toNumber() * 100;
  }

  /**
   *
   * @returns deposit rate, 0 is 0% where 1 is 100%
   */
  getDepositRate(): I80F48 {
    const borrowRate = this.getBorrowRate();
    const totalBorrows = this.nativeBorrows();
    const totalDeposits = this.nativeDeposits();

    if (totalDeposits.isZero() && totalBorrows.isZero()) {
      return ZERO_I80F48();
    } else if (totalDeposits.isZero()) {
      return this.maxRate;
    }

    const utilization = totalBorrows.div(totalDeposits);
    return utilization.mul(borrowRate);
  }

  /**
   *
   * @returns deposit rate percentage
   */
  getDepositRateUi(): number {
    return this.getDepositRate().toNumber() * 100;
  }
}

export class MintInfo {
  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      tokenIndex: number;
      mint: PublicKey;
      banks: PublicKey[];
      vaults: PublicKey[];
      oracle: PublicKey;
      registrationTime: BN;
      groupInsuranceFund: number;
    },
  ): MintInfo {
    return new MintInfo(
      publicKey,
      obj.group,
      obj.tokenIndex as TokenIndex,
      obj.mint,
      obj.banks,
      obj.vaults,
      obj.oracle,
      obj.registrationTime,
      obj.groupInsuranceFund == 1,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public tokenIndex: TokenIndex,
    public mint: PublicKey,
    public banks: PublicKey[],
    public vaults: PublicKey[],
    public oracle: PublicKey,
    public registrationTime: BN,
    public groupInsuranceFund: boolean,
  ) {}

  public firstBank(): PublicKey {
    return this.banks[0];
  }
  public firstVault(): PublicKey {
    return this.vaults[0];
  }

  toString(): string {
    const res =
      'mint ' +
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
