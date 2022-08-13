import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { nativeI80F48ToUi } from '../utils';
import { I80F48, I80F48Dto, ZERO_I80F48 } from './I80F48';

export const QUOTE_DECIMALS = 6;

export type OracleConfig = {
  confFilter: I80F48Dto;
};

export class Bank {
  public name: string;
  public depositIndex: I80F48;
  public borrowIndex: I80F48;
  public indexedDeposits: I80F48;
  public indexedBorrows: I80F48;
  public cachedIndexedTotalDeposits: I80F48;
  public cachedIndexedTotalBorrows: I80F48;
  public avgUtilization: I80F48;
  public adjustmentFactor: I80F48;
  public maxRate: I80F48;
  public rate0: I80F48;
  public rate1: I80F48;
  public util0: I80F48;
  public util1: I80F48;
  public price: I80F48;
  public collectedFeesNative: I80F48;
  public loanFeeRate: I80F48;
  public loanOriginationFeeRate: I80F48;
  public initAssetWeight: I80F48;
  public maintAssetWeight: I80F48;
  public initLiabWeight: I80F48;
  public maintLiabWeight: I80F48;
  public liquidationFee: I80F48;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      name: number[];
      mint: PublicKey;
      vault: PublicKey;
      oracle: PublicKey;
      oracleConfig: OracleConfig;
      depositIndex: I80F48Dto;
      borrowIndex: I80F48Dto;
      cachedIndexedTotalDeposits: I80F48Dto;
      cachedIndexedTotalBorrows: I80F48Dto;
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
      loanFeeRate: I80F48Dto;
      loanOriginationFeeRate: I80F48Dto;
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
    },
  ) {
    return new Bank(
      publicKey,
      obj.name,
      obj.group,
      obj.mint,
      obj.vault,
      obj.oracle,
      obj.oracleConfig,
      obj.depositIndex,
      obj.borrowIndex,
      obj.cachedIndexedTotalDeposits,
      obj.cachedIndexedTotalBorrows,
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
      obj.loanFeeRate,
      obj.loanOriginationFeeRate,
      obj.maintAssetWeight,
      obj.initAssetWeight,
      obj.maintLiabWeight,
      obj.initLiabWeight,
      obj.liquidationFee,
      obj.dust,
      obj.flashLoanTokenAccountInitial,
      obj.flashLoanApprovedAmount,
      obj.tokenIndex,
      obj.mintDecimals,
      obj.bankNum,
    );
  }

  constructor(
    public publicKey: PublicKey,
    name: number[],
    public group: PublicKey,
    public mint: PublicKey,
    public vault: PublicKey,
    public oracle: PublicKey,
    oracleConfig: OracleConfig,
    depositIndex: I80F48Dto,
    borrowIndex: I80F48Dto,
    indexedTotalDeposits: I80F48Dto,
    indexedTotalBorrows: I80F48Dto,
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
    loanFeeRate: I80F48Dto,
    loanOriginationFeeRate: I80F48Dto,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    liquidationFee: I80F48Dto,
    dust: I80F48Dto,
    flashLoanTokenAccountInitial: BN,
    flashLoanApprovedAmount: BN,
    public tokenIndex: number,
    public mintDecimals: number,
    public bankNum: number,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.depositIndex = I80F48.from(depositIndex);
    this.borrowIndex = I80F48.from(borrowIndex);
    this.indexedDeposits = I80F48.from(indexedDeposits);
    this.indexedBorrows = I80F48.from(indexedBorrows);
    this.cachedIndexedTotalDeposits = I80F48.from(indexedTotalDeposits);
    this.cachedIndexedTotalBorrows = I80F48.from(indexedTotalBorrows);
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
    this.price = undefined;
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
      '\n deposit index - ' +
      this.depositIndex.toNumber() +
      '\n borrow index - ' +
      this.borrowIndex.toNumber() +
      '\n indexedDeposits - ' +
      this.indexedDeposits.toNumber() +
      '\n indexedBorrows - ' +
      this.indexedBorrows.toNumber() +
      '\n cachedIndexedTotalDeposits - ' +
      this.cachedIndexedTotalDeposits.toNumber() +
      '\n cachedIndexedTotalBorrows - ' +
      this.cachedIndexedTotalBorrows.toNumber() +
      '\n indexLastUpdated - ' +
      new Date(this.indexLastUpdated.toNumber() * 1000) +
      '\n bankRateLastUpdated - ' +
      new Date(this.bankRateLastUpdated.toNumber() * 1000) +
      '\n avgUtilization - ' +
      this.avgUtilization.toNumber() +
      '\n adjustmentFactor - ' +
      this.adjustmentFactor.toNumber() +
      '\n maxRate - ' +
      this.maxRate.toNumber() +
      '\n util0 - ' +
      this.util0.toNumber() +
      '\n rate0 - ' +
      this.rate0.toNumber() +
      '\n util1 - ' +
      this.util1.toNumber() +
      '\n rate1 - ' +
      this.rate1.toNumber() +
      '\n loanFeeRate - ' +
      this.loanFeeRate.toNumber() +
      '\n loanOriginationFeeRate - ' +
      this.loanOriginationFeeRate.toNumber() +
      '\n maintAssetWeight - ' +
      this.maintAssetWeight.toNumber() +
      '\n initAssetWeight - ' +
      this.initAssetWeight.toNumber() +
      '\n maintLiabWeight - ' +
      this.maintLiabWeight.toNumber() +
      '\n initLiabWeight - ' +
      this.initLiabWeight.toNumber() +
      '\n liquidationFee - ' +
      this.liquidationFee.toNumber() +
      '\n uiDeposits() - ' +
      this.uiDeposits() +
      '\n uiBorrows() - ' +
      this.uiBorrows() +
      '\n getDepositRate() - ' +
      this.getDepositRate().toNumber() +
      '\n getBorrowRate() - ' +
      this.getBorrowRate().toNumber()
    );
  }

  nativeDeposits(): I80F48 {
    return this.cachedIndexedTotalDeposits.mul(this.depositIndex);
  }

  nativeBorrows(): I80F48 {
    return this.cachedIndexedTotalBorrows.mul(this.borrowIndex);
  }

  uiDeposits(): number {
    return nativeI80F48ToUi(
      this.cachedIndexedTotalDeposits.mul(this.depositIndex),
      this.mintDecimals,
    ).toNumber();
  }

  uiBorrows(): number {
    return nativeI80F48ToUi(
      this.cachedIndexedTotalBorrows.mul(this.borrowIndex),
      this.mintDecimals,
    ).toNumber();
  }

  getBorrowRate(): I80F48 {
    const totalBorrows = this.nativeBorrows();
    const totalDeposits = this.nativeDeposits();

    if (totalDeposits.eq(ZERO_I80F48) && totalBorrows.eq(ZERO_I80F48)) {
      return ZERO_I80F48;
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
    } else if (utilization.gt(this.util0)) {
      const extraUtil = utilization.sub(this.util0);
      const slope = this.maxRate
        .sub(this.rate0)
        .div(I80F48.fromNumber(1).sub(this.util0));
      return this.rate0.add(slope.mul(extraUtil));
    } else {
      const slope = this.rate0.div(this.util0);
      return slope.mul(utilization);
    }
  }

  getDepositRate(): I80F48 {
    const borrowRate = this.getBorrowRate();
    const totalBorrows = this.nativeBorrows();
    const totalDeposits = this.nativeDeposits();

    if (totalDeposits.eq(ZERO_I80F48) && totalBorrows.eq(ZERO_I80F48)) {
      return ZERO_I80F48;
    } else if (totalDeposits.eq(ZERO_I80F48)) {
      return this.maxRate;
    }

    const utilization = totalBorrows.div(totalDeposits);
    return utilization.mul(borrowRate);
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
  ) {
    return new MintInfo(
      publicKey,
      obj.group,
      obj.tokenIndex,
      obj.mint,
      obj.banks,
      obj.vaults,
      obj.oracle,
      obj.registrationTime,
      obj.groupInsuranceFund,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public tokenIndex: number,
    public mint: PublicKey,
    public banks: PublicKey[],
    public vaults: PublicKey[],
    public oracle: PublicKey,
    public registrationTime: BN,
    public groupInsuranceFund: number,
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
