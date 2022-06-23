import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../client';
import { nativeI80F48ToUi } from '../utils';
import { Bank } from './bank';
import { Group } from './group';
import { I80F48, I80F48Dto, ZERO_I80F48 } from './I80F48';
export class MangoAccount {
  public tokens: TokenPosition[];
  public serum3: Serum3Orders[];
  public perps: PerpPositions[];
  public name: string;

  static from(
    publicKey: PublicKey,
    obj: {
      name: number[];
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
      obj.name,
      obj.group,
      obj.owner,
      obj.delegate,
      obj.tokens as { values: TokenAccountDto[] },
      obj.serum3 as { values: Serum3AccountDto[] },
      obj.perps as { accounts: PerpAccountDto[] },
      obj.beingLiquidated,
      obj.isBankrupt,
      obj.accountNum,
      obj.bump,
      obj.reserved,
    );
  }

  constructor(
    public publicKey: PublicKey,
    name: number[],
    public group: PublicKey,
    public owner: PublicKey,
    public delegate: PublicKey,
    tokens: { values: TokenAccountDto[] },
    serum3: { values: Serum3AccountDto[] },
    perps: { accounts: PerpAccountDto[] },
    beingLiquidated: number,
    isBankrupt: number,
    accountNum: number,
    bump: number,
    reserved: number[],
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.tokens = tokens.values.map((dto) => TokenPosition.from(dto));
    this.serum3 = serum3.values.map((dto) => Serum3Orders.from(dto));
    this.perps = perps.accounts.map((dto) => PerpPositions.from(dto));
  }

  async reload(client: MangoClient) {
    Object.assign(this, await client.getMangoAccount(this));
  }

  findToken(tokenIndex: number): TokenPosition | undefined {
    return this.tokens.find((ta) => ta.tokenIndex == tokenIndex);
  }

  findSerum3Account(marketIndex: number): Serum3Orders | undefined {
    return this.serum3.find((sa) => sa.marketIndex == marketIndex);
  }

  getNative(bank: Bank): I80F48 {
    const ta = this.findToken(bank.tokenIndex);
    return ta ? ta.native(bank) : ZERO_I80F48;
  }

  getUi(bank: Bank): number {
    const ta = this.findToken(bank.tokenIndex);
    return ta ? ta.ui(bank) : 0;
  }

  tokens_active(): TokenPosition[] {
    return this.tokens.filter((token) => token.isActive());
  }

  toString(group?: Group): string {
    return (
      'tokens:' +
      JSON.stringify(
        this.tokens
          .filter((token) => token.tokenIndex != TokenPosition.TokenIndexUnset)
          .map((token) => token.toString(group)),
        null,
        4,
      ) +
      '\nserum:' +
      JSON.stringify(
        this.serum3.filter(
          (serum3) => serum3.marketIndex != Serum3Orders.Serum3MarketIndexUnset,
        ),
        null,
        4,
      ) +
      '\nperps:' +
      JSON.stringify(
        this.perps.filter(
          (perp) => perp.marketIndex != PerpPositions.PerpMarketIndexUnset,
        ),
        null,
        4,
      )
    );
  }
}

export class TokenPosition {
  static TokenIndexUnset: number = 65535;
  static from(dto: TokenAccountDto) {
    return new TokenPosition(
      I80F48.from(dto.indexedPosition),
      dto.tokenIndex,
      dto.inUseCount,
    );
  }

  constructor(
    public indexedPosition: I80F48,
    public tokenIndex: number,
    public inUseCount: number,
  ) {}

  public isActive(): boolean {
    return this.tokenIndex !== 65535;
  }

  public native(bank: Bank): I80F48 {
    if (this.indexedPosition.isPos()) {
      return bank.depositIndex.mul(this.indexedPosition);
    } else {
      return bank.borrowIndex.mul(this.indexedPosition);
    }
  }

  public ui(bank: Bank): number {
    return nativeI80F48ToUi(this.native(bank), bank.mintDecimals).toNumber();
  }

  public toString(group?: Group): String {
    let extra: string = '';
    if (group) {
      let bank = group.findBank(this.tokenIndex);
      if (bank) {
        let native = this.native(bank);
        extra += ', native: ' + native.toNumber();
        extra += ', ui: ' + this.ui(bank);
        extra += ', tokenName: ' + bank.name;
      }
    }

    return (
      'tokenIndex: ' +
      this.tokenIndex +
      ', inUseCount: ' +
      this.inUseCount +
      ', indexedValue: ' +
      this.indexedPosition.toNumber() +
      extra
    );
  }
}

export class TokenAccountDto {
  constructor(
    public indexedPosition: I80F48Dto,
    public tokenIndex: number,
    public inUseCount: number,
    public reserved: number[],
  ) {}
}

export class Serum3Orders {
  static Serum3MarketIndexUnset = 65535;
  static from(dto: Serum3AccountDto) {
    return new Serum3Orders(
      dto.openOrders,
      dto.marketIndex,
      dto.baseTokenIndex,
      dto.quoteTokenIndex,
    );
  }

  constructor(
    public openOrders: PublicKey,
    public marketIndex: number,
    public baseTokenIndex: number,
    public quoteTokenIndex: number,
  ) {}
}

export class Serum3AccountDto {
  constructor(
    public openOrders: PublicKey,
    public marketIndex: number,
    public baseTokenIndex: number,
    public quoteTokenIndex: number,
    public reserved: number[],
  ) {}
}

export class PerpPositions {
  static PerpMarketIndexUnset = 65535;
  static from(dto: PerpAccountDto) {
    return new PerpPositions(
      dto.marketIndex,
      dto.basePositionLots.toNumber(),
      dto.quotePositionNative.val.toNumber(),
      dto.bidsBaseLots.toNumber(),
      dto.asksBaseLots.toNumber(),
      dto.takerBaseLots.toNumber(),
      dto.takerQuoteLots.toNumber(),
    );
  }

  constructor(
    public marketIndex: number,
    public basePositionLots: number,
    public quotePositionNative: number,
    public bidsBaseLots: number,
    public asksBaseLots: number,
    public takerBaseLots: number,
    public takerQuoteLots: number,
  ) {}
}

export class PerpAccountDto {
  constructor(
    public marketIndex: number,
    public reserved: [],
    public basePositionLots: BN,
    public quotePositionNative: { val: BN },
    public bidsBaseLots: BN,
    public asksBaseLots: BN,
    public takerBaseLots: BN,
    public takerQuoteLots: BN,
  ) {}
}
