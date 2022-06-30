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
      obj.tokens as { values: TokenPositionDto[] },
      obj.serum3 as { values: Serum3PositionDto[] },
      obj.perps as { accounts: PerpPositionDto[] },
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
    tokens: { values: TokenPositionDto[] },
    serum3: { values: Serum3PositionDto[] },
    perps: { accounts: PerpPositionDto[] },
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

  deposits(bank: Bank): number {
    const ta = this.findToken(bank.tokenIndex);
    return ta ? ta.uiDeposits(bank) : 0;
  }

  borrows(bank: Bank): number {
    const ta = this.findToken(bank.tokenIndex);
    return ta ? ta.uiBorrows(bank) : 0;
  }

  tokensActive(): TokenPosition[] {
    return this.tokens.filter((token) => token.isActive());
  }

  serum3Active(): Serum3Orders[] {
    return this.serum3.filter((serum3) => serum3.isActive());
  }

  perpActive(): PerpPositions[] {
    return this.perps.filter((perp) => perp.isActive());
  }

  toString(group?: Group): string {
    let res = '';
    res = res + ' name: ' + this.name;

    res =
      this.tokensActive().length > 0
        ? res +
          '\n tokens:' +
          JSON.stringify(
            this.tokensActive().map((token) => token.toString(group)),
            null,
            4,
          )
        : res + '';

    res =
      this.serum3Active().length > 0
        ? res + '\n serum:' + JSON.stringify(this.serum3Active(), null, 4)
        : res + '';

    res =
      this.perpActive().length > 0
        ? res + '\n perps:' + JSON.stringify(this.perpActive(), null, 4)
        : res + '';

    return res;
  }
}

export class TokenPosition {
  static TokenIndexUnset: number = 65535;
  static from(dto: TokenPositionDto) {
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

  public uiDeposits(bank: Bank): number {
    return nativeI80F48ToUi(
      bank.depositIndex.mul(this.indexedPosition),
      bank.mintDecimals,
    ).toNumber();
  }

  public uiBorrows(bank: Bank): number {
    return nativeI80F48ToUi(
      bank.borrowIndex.mul(this.indexedPosition),
      bank.mintDecimals,
    ).toNumber();
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

export class TokenPositionDto {
  constructor(
    public indexedPosition: I80F48Dto,
    public tokenIndex: number,
    public inUseCount: number,
    public reserved: number[],
  ) {}
}

export class Serum3Orders {
  static Serum3MarketIndexUnset = 65535;
  static from(dto: Serum3PositionDto) {
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

  public isActive(): boolean {
    return this.marketIndex !== Serum3Orders.Serum3MarketIndexUnset;
  }
}

export class Serum3PositionDto {
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
  static from(dto: PerpPositionDto) {
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

  isActive(): boolean {
    return this.marketIndex != PerpPositions.PerpMarketIndexUnset;
  }
}

export class PerpPositionDto {
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
