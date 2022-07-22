import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../client';
import { nativeI80F48ToUi } from '../utils';
import { Bank, QUOTE_DECIMALS } from './bank';
import { Group } from './group';
import { HealthCache, HealthCacheDto } from './healthCache';
import { I80F48, I80F48Dto, ONE_I80F48, ZERO_I80F48 } from './I80F48';
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
      perpOpenOrders: unknown;
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
      obj.tokens as TokenPositionDto[],
      obj.serum3 as Serum3PositionDto[],
      obj.perps as PerpPositionDto[],
      obj.perpOpenOrders as any, // TODO
      obj.beingLiquidated,
      obj.isBankrupt,
      obj.accountNum,
      obj.bump,
      obj.reserved,
      {},
    );
  }

  constructor(
    public publicKey: PublicKey,
    name: number[],
    public group: PublicKey,
    public owner: PublicKey,
    public delegate: PublicKey,
    tokens: TokenPositionDto[],
    serum3: Serum3PositionDto[],
    perps: PerpPositionDto[],
    perpOpenOrders: PerpPositionDto[],
    beingLiquidated: number,
    isBankrupt: number,
    accountNum: number,
    bump: number,
    reserved: number[],
    public accountData: {},
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.tokens = tokens.map((dto) => TokenPosition.from(dto));
    this.serum3 = serum3.map((dto) => Serum3Orders.from(dto));
    this.perps = perps.map((dto) => PerpPositions.from(dto));
  }

  async reload(client: MangoClient, group: Group) {
    Object.assign(this, await client.getMangoAccount(this));
    await this.reloadAccountData(client, group);
  }

  async reloadAccountData(client: MangoClient, group: Group) {
    this.accountData = await client.computeAccountData(group, this);
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

  static getEquivalentNativeUsdcPosition(
    sourceBank: Bank,
    nativeTokenPosition: TokenPosition,
  ): I80F48 {
    return nativeTokenPosition
      .native(sourceBank)
      .mul(I80F48.fromNumber(Math.pow(10, QUOTE_DECIMALS)))
      .div(I80F48.fromNumber(Math.pow(10, sourceBank.mintDecimals)))
      .mul(sourceBank.price);
  }

  static getEquivalentNativeTokenPosition(
    targetBank: Bank,
    nativeUsdcPosition: I80F48,
  ): I80F48 {
    return nativeUsdcPosition
      .div(targetBank.price)
      .div(I80F48.fromNumber(Math.pow(10, QUOTE_DECIMALS)))
      .mul(I80F48.fromNumber(Math.pow(10, targetBank.mintDecimals)));
  }

  getNativeDeposits(bank: Bank): I80F48 {
    const native = this.getNative(bank);
    return native.gte(ZERO_I80F48) ? native : ZERO_I80F48;
  }

  getNativeBorrows(bank: Bank): I80F48 {
    const native = this.getNative(bank);
    return native.lte(ZERO_I80F48) ? native : ZERO_I80F48;
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

  getHealth(healthType: HealthType): I80F48 {
    return healthType == HealthType.init
      ? (this.accountData as MangoAccountData).initHealth
      : (this.accountData as MangoAccountData).maintHealth;
  }

  /**
   * TODO: this is incorrect, getAssetsVal and getLiabsVal are in equity, and not in given health type.
   * Wait for dev to be deployed to mainnet, and then we can adapt this.
   */
  getHealthRatio(healthType: HealthType): I80F48 {
    const assets = this.getAssetsVal();
    const liabs = this.getLiabsVal();
    return liabs.gt(ZERO_I80F48)
      ? assets.div(liabs).sub(ONE_I80F48).mul(I80F48.fromNumber(100))
      : I80F48.fromNumber(100);
  }

  /**
   * Sum of all the assets i.e. token deposits, borrows, total assets in spot open orders, (perps positions is todo) in terms of quote value.
   */
  getEquity(): I80F48 {
    const equity = (this.accountData as MangoAccountData).equity;
    let total_equity = equity.tokens.reduce(
      (a, b) => a.add(b.value),
      ZERO_I80F48,
    );
    return total_equity;
  }

  /**
   * The amount of native quote you could withdraw against your existing assets.
   */
  getCollateralValue(): I80F48 {
    return this.getHealth(HealthType.init);
  }

  /**
   * Similar to getEquity, but only the sum of all positive assets.
   */
  getAssetsVal(): I80F48 {
    const equity = (this.accountData as MangoAccountData).equity;
    let total_equity = equity.tokens.reduce(
      (a, b) => (b.value.gt(ZERO_I80F48) ? a.add(b.value) : a),
      ZERO_I80F48,
    );
    return total_equity;
  }

  /**
   * Similar to getEquity, but only the sum of all negative assets. Note: return value would be negative.
   */
  getLiabsVal(): I80F48 {
    const equity = (this.accountData as MangoAccountData).equity;
    let total_equity = equity.tokens.reduce(
      (a, b) => (b.value.lt(ZERO_I80F48) ? a.add(b.value) : a),
      ZERO_I80F48,
    );
    return total_equity;
  }

  /**
   * The amount of given native token you can borrow, considering all existing assets as collateral except the deposits for this token.
   * Note 1: The existing native deposits need to be added to get the full amount that could be withdrawn.
   * Note 2: The group might have less native deposits than what this returns.
   */
  getMaxWithdrawWithBorrowForToken(group: Group, tokenName: string): I80F48 {
    const bank = group.banksMap.get(tokenName);
    const initHealth = (this.accountData as MangoAccountData).initHealth;
    const inUsdcUnits = MangoAccount.getEquivalentNativeUsdcPosition(
      bank,
      this.findToken(bank.tokenIndex),
    );
    const newInitHealth = initHealth.sub(inUsdcUnits.mul(bank.initAssetWeight));
    return MangoAccount.getEquivalentNativeTokenPosition(
      bank,
      newInitHealth.div(bank.initLiabWeight),
    );
  }

  /**
   * The amount of given source native token you can swap to a target token considering all existing assets as collateral.
   * note: slippageAndFeesFactor is a normalized number, <1, e.g. a slippage of 5% and some fees which are 1%, then slippageAndFeesFactor = 0.94
   * the factor is used to compute how much target can be obtained by swapping source
   */
  getMaxSourceForTokenSwap(
    group: Group,
    sourceTokenName: string,
    targetTokenName: string,
    slippageAndFeesFactor: number,
  ): I80F48 {
    const initHealth = (this.accountData as MangoAccountData).initHealth;

    const sourceBank = group.banksMap.get(sourceTokenName);
    const targetBank = group.banksMap.get(targetTokenName);

    // This is a conservative approximation of the easy case, where
    // mango account has no token positions for source and target tokens, or
    // borrows for source and deposits for target tokens before the swap.
    // Tighter estimates can be obtained by adding cases where deposits can exist for source,
    // and borrows for target. TODO: solve this by searching over a blackbox like health formula.
    // Lets solve below for s,
    // h - s * slw + t * taw = 0
    // where h is init_health, s is source amount in usdc native units, and t is target amount in usdc native units
    // where t = s * slip ( s < 1), where slip is factor for slippage and fees which is normalised e.g. for 5% slippage, slip = 0.95
    // h - s * (slw - slip * taw) = 0
    // s = h / ( slw - slip * taw )
    return initHealth
      .div(
        sourceBank.initLiabWeight.sub(
          I80F48.fromNumber(slippageAndFeesFactor).mul(
            targetBank.initAssetWeight,
          ),
        ),
      )
      .div(sourceBank.price);
  }

  /**
   * Simulates new health after applying tokenChanges to the token positions. Useful to simulate health after a potential swap.
   */
  simHealthWithTokenPositionChanges(
    group: Group,
    tokenChanges: { tokenName: string; tokenAmount: number }[],
  ): I80F48 {
    // This is a approximation of the easy case, where
    // mango account has no token positions for tokens in changes list, or
    // the change is in direction e.g. deposits for deposits, borrows for borrows, of existing token position.
    // TODO: recompute entire health using components.
    const initHealth = (this.accountData as MangoAccountData).initHealth;
    for (const change of tokenChanges) {
      const bank = group.banksMap.get(change.tokenName);
      if (change.tokenAmount >= 0) {
        initHealth.add(
          bank.initAssetWeight
            .mul(I80F48.fromNumber(change.tokenAmount))
            .mul(bank.price),
        );
      } else {
        initHealth.sub(
          bank.initLiabWeight
            .mul(I80F48.fromNumber(change.tokenAmount))
            .mul(bank.price),
        );
      }
    }
    return initHealth;
  }

  /**
   * The remaining native quote margin available for given market.
   *
   * TODO: this is a very bad estimation atm.
   * It assumes quote asset is always USDC,
   * it assumes that there are no interaction effects,
   * it assumes that there are no existing borrows for either of the tokens in the market.
   */
  getSerum3MarketMarginAvailable(group: Group, marketName: string): I80F48 {
    const initHealth = (this.accountData as MangoAccountData).initHealth;
    const serum3Market = group.serum3MarketsMap.get(marketName)!;
    const marketAssetWeight = group.findBank(
      serum3Market.baseTokenIndex,
    ).initAssetWeight;
    return initHealth.div(ONE_I80F48.sub(marketAssetWeight));
  }

  /**
   * The remaining native quote margin available for given market.
   *
   * TODO: this is a very bad estimation atm.
   * It assumes quote asset is always USDC,
   * it assumes that there are no interaction effects,
   * it assumes that there are no existing borrows for either of the tokens in the market.
   */
  getPerpMarketMarginAvailable(group: Group, marketName: string): I80F48 {
    const initHealth = (this.accountData as MangoAccountData).initHealth;
    const perpMarket = group.perpMarketsMap.get(marketName)!;
    const marketAssetWeight = perpMarket.initAssetWeight;
    return initHealth.div(ONE_I80F48.sub(marketAssetWeight));
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
    let res = 'MangoAccount';
    res = res + '\n pk: ' + this.publicKey.toString();
    res = res + '\n name: ' + this.name;
    res = res + '\n delegate: ' + this.delegate;

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

export class HealthType {
  static maint = { maint: {} };
  static init = { init: {} };
}

export class MangoAccountData {
  constructor(
    public healthCache: HealthCache,
    public initHealth: I80F48,
    public maintHealth: I80F48,
    public equity: Equity,
  ) {}

  static from(event: {
    healthCache: HealthCacheDto;
    initHealth: I80F48Dto;
    maintHealth: I80F48Dto;
    equity: {
      tokens: [{ tokenIndex: number; value: I80F48Dto }];
      perps: [{ perpMarketIndex: number; value: I80F48Dto }];
    };
    initHealthLiabs: I80F48Dto;
    tokenAssets: any;
  }) {
    return new MangoAccountData(
      new HealthCache(event.healthCache),
      I80F48.from(event.initHealth),
      I80F48.from(event.maintHealth),
      Equity.from(event.equity),
    );
  }
}

export class Equity {
  public constructor(
    public tokens: TokenEquity[],
    public perps: PerpEquity[],
  ) {}

  static from(dto: EquityDto): Equity {
    return new Equity(
      dto.tokens.map(
        (token) => new TokenEquity(token.tokenIndex, I80F48.from(token.value)),
      ),
      dto.perps.map(
        (perpAccount) =>
          new PerpEquity(
            perpAccount.perpMarketIndex,
            I80F48.from(perpAccount.value),
          ),
      ),
    );
  }
}

export class TokenEquity {
  public constructor(public tokenIndex: number, public value: I80F48) {}
}

export class PerpEquity {
  public constructor(public perpMarketIndex: number, public value: I80F48) {}
}

export class EquityDto {
  tokens: { tokenIndex: number; value: I80F48Dto }[];
  perps: { perpMarketIndex: number; value: I80F48Dto }[];
}

export class AccountSize {
  static small = { small: {} };
  static large = { large: {} };
}
