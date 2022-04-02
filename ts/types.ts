import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import BN from 'bn.js';
import { I80F48 } from './I80F48';

export class Group {
  static from(publicKey: PublicKey, obj: { admin: PublicKey }): Group {
    return new Group(publicKey, obj.admin);
  }
  constructor(public publicKey: PublicKey, public admin: PublicKey) {}
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
  public tokens: TokenAccount[];

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      owner: PublicKey;
      delegate: PublicKey;
      tokens: unknown;
      serum3: Object;
      perps: unknown;
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
      obj.tokens as { values: TokenAccountDto[] },
      obj.serum3,
      obj.perps,
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
    tokens: { values: TokenAccountDto[] },
    serum3: Object,
    perps: unknown,
    beingLiquidated: number,
    isBankrupt: number,
    accountNum: number,
    bump: number,
    reserved: number[],
  ) {
    this.tokens = tokens.values.map((dto) => TokenAccount.from(dto));
  }

  find(tokenIndex: number): TokenAccount | undefined {
    return this.tokens.find((ta) => ta.tokenIndex == tokenIndex);
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

export class Serum3Market {
  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      serumProgram: PublicKey;
      serumMarketExternal: PublicKey;
      marketIndex: number;
      baseTokenIndex: number;
      quoteTokenIndex: number;
      bump: number;
      reserved: unknown;
    },
  ): Serum3Market {
    return new Serum3Market(
      publicKey,
      obj.group,
      obj.serumProgram,
      obj.serumMarketExternal,
      obj.marketIndex,
      obj.baseTokenIndex,
      obj.quoteTokenIndex,
    );
  }
  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public serumProgram: PublicKey,
    public serumMarketExternal: PublicKey,
    public marketIndex: number,
    public baseTokenIndex: number,
    public quoteTokenIndex: number,
  ) {}
}
