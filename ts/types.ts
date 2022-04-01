import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import BN from 'bn.js';
import { I80F48 } from './I80F48';

export class Group {
  constructor(admin: PublicKey, bump: number) {}
}

export class Bank {
  public depositIndex: I80F48;
  public borrowIndex: I80F48;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      mint: PublicKey;
      vault: PublicKey;
      oracle: PublicKey;
      depositIndex: I80F48Dto;
      borrowIndex: I80F48Dto;
      indexedTotalDeposits: I80F48Dto;
      indexedTotalBorrows: I80F48Dto;
      maintAssetWeight: I80F48Dto;
      initAssetWeight: I80F48Dto;
      maintLiabWeight: I80F48Dto;
      initLiabWeight: I80F48Dto;
      liquidationFee: I80F48Dto;
      dust: Object;
      tokenIndex: number;
    },
  ) {
    return new Bank(
      publicKey,
      obj.group,
      obj.mint,
      obj.vault,
      obj.oracle,
      obj.depositIndex,
      obj.borrowIndex,
      obj.indexedTotalDeposits,
      obj.indexedTotalBorrows,
      obj.maintAssetWeight,
      obj.initAssetWeight,
      obj.maintLiabWeight,
      obj.initLiabWeight,
      obj.liquidationFee,
      obj.dust,
      obj.tokenIndex,
    );
  }

  constructor(
    public publicKey: PublicKey,
    group: PublicKey,
    mint: PublicKey,
    public vault: PublicKey,
    oracle: PublicKey,
    depositIndex: I80F48Dto,
    borrowIndex: I80F48Dto,
    indexedTotalDeposits: I80F48Dto,
    indexedTotalBorrows: I80F48Dto,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    liquidationFee: I80F48Dto,
    dust: Object,
    public tokenIndex: number,
  ) {
    this.depositIndex = I80F48.from(depositIndex);
    this.borrowIndex = I80F48.from(borrowIndex);
  }

  toString(): string {
    return `Bank ${
      this.tokenIndex
    } deposit index - ${this.depositIndex.toNumber()}, borrow index - ${this.borrowIndex.toNumber()}`;
  }
}

export class MangoAccount {
  public tokenAccountMap: TokenAccount[];

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      owner: PublicKey;
      delegate: PublicKey;
      tokenAccountMap: unknown;
      serum3AccountMap: Object;
      perpAccountMap: Object;
      orderMarket: number[];
      orderSide: unknown;
      orders: BN[];
      clientOrderIds: BN[];
      beingLiquidated: number;
      isBankrupt: number;
      accountNum: number;
      bump: number;
      reserved: number[];
    },
  ) {
    return new MangoAccount(
      publicKey,
      obj.group,
      obj.owner,
      obj.delegate,
      obj.tokenAccountMap as { values: TokenAccountDto[] },
      obj.serum3AccountMap,
      obj.perpAccountMap,
      obj.orderMarket,
      obj.orderSide,
      obj.orders,
      obj.clientOrderIds,
      obj.beingLiquidated,
      obj.isBankrupt,
      obj.accountNum,
      obj.bump,
      obj.reserved,
    );
  }

  constructor(
    public publicKey: PublicKey,
    group: PublicKey,
    owner: PublicKey,
    delegate: PublicKey,
    tokenAccountMap: { values: TokenAccountDto[] },
    serum3AccountMap: Object,
    perpAccountMap: Object,
    orderMarket: number[],
    orderSide: unknown,
    orders: BN[],
    clientOrderIds: BN[],
    beingLiquidated: number,
    isBankrupt: number,
    accountNum: number,
    bump: number,
    reserved: number[],
  ) {
    this.tokenAccountMap = tokenAccountMap.values.map((dto) =>
      TokenAccount.from(dto),
    );
  }

  find(tokenIndex: number): TokenAccount | undefined {
    return this.tokenAccountMap.find((ta) => ta.tokenIndex == tokenIndex);
  }

  getNativeDeposit(bank: Bank): I80F48 {
    const ta = this.find(bank.tokenIndex);
    return bank.depositIndex.mul(ta?.indexedValue!);
  }

  getNativeBorrow(bank: Bank): I80F48 {
    const ta = this.find(bank.tokenIndex);
    return bank.borrowIndex.mul(ta?.indexedValue!);
  }
}

export class I80F48Dto {
  constructor(public val: BN) {}
}

export class TokenAccount {
  static from(dto: TokenAccountDto) {
    return new TokenAccount(
      I80F48.from(dto.indexedValue),
      dto.tokenIndex,
      dto.inUseCount,
    );
  }

  constructor(
    public indexedValue: I80F48,
    public tokenIndex: number,
    public inUseCount: number,
  ) {}
}
export class TokenAccountDto {
  constructor(
    public indexedValue: I80F48Dto,
    public tokenIndex: number,
    public inUseCount: number,
    public reserved: number[],
  ) {}
}
