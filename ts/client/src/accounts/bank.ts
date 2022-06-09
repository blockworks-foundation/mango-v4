import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { I80F48, I80F48Dto } from './I80F48';

export const QUOTE_DECIMALS = 6;

export class Bank {
  public name: string;
  public depositIndex: I80F48;
  public borrowIndex: I80F48;
  public indexedTotalDeposits: I80F48;
  public indexedTotalBorrows: I80F48;

  static from(
    publicKey: PublicKey,
    obj: {
      name: number[];
      group: PublicKey;
      mint: PublicKey;
      vault: PublicKey;
      oracle: PublicKey;
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
    depositIndex: I80F48Dto,
    borrowIndex: I80F48Dto,
    indexedTotalDeposits: I80F48Dto,
    indexedTotalBorrows: I80F48Dto,
    last_updated: BN,
    util0: I80F48Dto,
    rate0: I80F48Dto,
    util1: I80F48Dto,
    rate1: I80F48Dto,
    max_rate: I80F48Dto,
    collected_fees_native: I80F48Dto,
    loan_origination_fee_rate: I80F48Dto,
    loan_fee_rate: I80F48Dto,
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
  }

  toString(): string {
    return `Bank ${
      this.tokenIndex
    } deposit index - ${this.depositIndex.toNumber()}, borrow index - ${this.borrowIndex.toNumber()}`;
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
      tokenIndex: Number;
      addressLookupTableBankIndex: Number;
      addressLookupTableOracleIndex: Number;
      reserved: unknown;
    },
  ) {
    return new MintInfo(publicKey, obj.mint, obj.bank, obj.vault, obj.oracle);
  }

  constructor(
    public publicKey: PublicKey,
    public mint: PublicKey,
    public bank: PublicKey,
    public vault: PublicKey,
    public oracle: PublicKey,
  ) {}
}
