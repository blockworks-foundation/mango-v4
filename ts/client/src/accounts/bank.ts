import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PythHttpClient } from '@pythnetwork/client';
import { PublicKey } from '@solana/web3.js';
import { nativeI80F48ToUi } from '../utils';
import { I80F48, I80F48Dto, ZERO_I80F48 } from './I80F48';

export const QUOTE_DECIMALS = 6;

type OracleConfig = {
  confFilter: I80F48Dto;
};

export class Bank {
  public name: string;
  public depositIndex: I80F48;
  public borrowIndex: I80F48;
  public indexedTotalDeposits: I80F48;
  public indexedTotalBorrows: I80F48;
  public maxRate: I80F48;
  public rate0: I80F48;
  public rate1: I80F48;
  public util0: I80F48;
  public util1: I80F48;
  public price: number;

  static from(
    publicKey: PublicKey,
    obj: {
      name: number[];
      group: PublicKey;
      mint: PublicKey;
      vault: PublicKey;
      oracle: PublicKey;
      oracleConfig: OracleConfig;
      depositIndex: I80F48Dto;
      borrowIndex: I80F48Dto;
      indexedTotalDeposits: I80F48Dto;
      indexedTotalBorrows: I80F48Dto;
      lastUpdated: BN;
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
      dust: Object;
      tokenIndex: number;
      mintDecimals: number;
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
      obj.indexedTotalDeposits,
      obj.indexedTotalBorrows,
      obj.lastUpdated,
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
      obj.tokenIndex,
      obj.mintDecimals,
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
    lastUpdated: BN,
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
    dust: Object,
    public tokenIndex: number,
    public mintDecimals: number,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.depositIndex = I80F48.from(depositIndex);
    this.borrowIndex = I80F48.from(borrowIndex);
    this.indexedTotalDeposits = I80F48.from(indexedTotalDeposits);
    this.indexedTotalBorrows = I80F48.from(indexedTotalBorrows);
    this.maxRate = I80F48.from(maxRate);
    this.util0 = I80F48.from(util0);
    this.rate0 = I80F48.from(rate0);
    this.util1 = I80F48.from(util1);
    this.rate1 = I80F48.from(rate1);
    this.price = undefined;
  }

  toString(): string {
    return `Bank ${
      this.tokenIndex
    } deposit index - ${this.depositIndex.toNumber()}, borrow index - ${this.borrowIndex.toNumber()}`;
  }

  nativeDeposits(): I80F48 {
    return this.indexedTotalDeposits.mul(this.depositIndex);
  }

  nativeBorrows(): I80F48 {
    return this.indexedTotalBorrows.mul(this.borrowIndex);
  }

  uiDeposits(): number {
    return nativeI80F48ToUi(
      this.indexedTotalDeposits.mul(this.depositIndex),
      this.mintDecimals,
    ).toNumber();
  }

  uiBorrows(): number {
    return nativeI80F48ToUi(
      this.indexedTotalBorrows.mul(this.borrowIndex),
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

  async getOraclePrice(connection) {
    const pythClient = new PythHttpClient(connection, this.oracle);
    const data = await pythClient.getData();

    return data.productPrice;
  }
}

export class MintInfo {
  static from(
    publicKey: PublicKey,
    obj: {
      mint: PublicKey;
      bank: PublicKey;
      vault: PublicKey;
      oracle: PublicKey;
      addressLookupTable: PublicKey;
      tokenIndex: number;
      addressLookupTableBankIndex: Number;
      addressLookupTableOracleIndex: Number;
      reserved: unknown;
    },
  ) {
    return new MintInfo(
      publicKey,
      obj.mint,
      obj.bank,
      obj.vault,
      obj.oracle,
      obj.tokenIndex,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public mint: PublicKey,
    public bank: PublicKey,
    public vault: PublicKey,
    public oracle: PublicKey,
    public tokenIndex: number,
  ) {}
}
