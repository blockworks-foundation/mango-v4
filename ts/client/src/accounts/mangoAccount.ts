import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { Order, Orderbook } from '@project-serum/serum/lib/market';
import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../client';
import { nativeI80F48ToUi, toNative, toUiDecimals } from '../utils';
import { Bank } from './bank';
import { Group } from './group';
import { HealthCache, HealthCacheDto } from './healthCache';
import { I80F48, I80F48Dto, ONE_I80F48, ZERO_I80F48 } from './I80F48';
import { PerpOrder } from './perp';
import { Serum3Market, Serum3Side } from './serum3';
export class MangoAccount {
  public tokens: TokenPosition[];
  public serum3: Serum3Orders[];
  public perps: PerpPosition[];
  public perpOpenOrders: PerpOo[];
  public name: string;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      owner: PublicKey;
      name: number[];
      delegate: PublicKey;
      beingLiquidated: number;
      accountNum: number;
      bump: number;
      netDeposits: BN;
      netSettled: BN;
      headerVersion: number;
      tokens: unknown;
      serum3: unknown;
      perps: unknown;
      perpOpenOrders: unknown;
    },
  ) {
    return new MangoAccount(
      publicKey,
      obj.group,
      obj.owner,
      obj.name,
      obj.delegate,
      obj.beingLiquidated,
      obj.accountNum,
      obj.bump,
      obj.netDeposits,
      obj.netSettled,
      obj.headerVersion,
      obj.tokens as TokenPositionDto[],
      obj.serum3 as Serum3PositionDto[],
      obj.perps as PerpPositionDto[],
      obj.perpOpenOrders as PerpOoDto[],
      {} as any,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public owner: PublicKey,
    name: number[],
    public delegate: PublicKey,
    beingLiquidated: number,
    public accountNum: number,
    bump: number,
    netDeposits: BN,
    netSettled: BN,
    headerVersion: number,
    tokens: TokenPositionDto[],
    serum3: Serum3PositionDto[],
    perps: PerpPositionDto[],
    perpOpenOrders: PerpOoDto[],
    public accountData: undefined | MangoAccountData,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.tokens = tokens.map((dto) => TokenPosition.from(dto));
    this.serum3 = serum3.map((dto) => Serum3Orders.from(dto));
    this.perps = perps.map((dto) => PerpPosition.from(dto));
    this.perpOpenOrders = perpOpenOrders.map((dto) => PerpOo.from(dto));
    this.accountData = undefined;
  }

  async reload(client: MangoClient, group: Group): Promise<MangoAccount> {
    const mangoAccount = await client.getMangoAccount(this);
    await mangoAccount.reloadAccountData(client, group);
    Object.assign(this, mangoAccount);
    return mangoAccount;
  }

  async reloadWithSlot(
    client: MangoClient,
    group: Group,
  ): Promise<{ value: MangoAccount; slot: number }> {
    const resp = await client.getMangoAccountWithSlot(this.publicKey);
    await resp?.value.reloadAccountData(client, group);
    Object.assign(this, resp?.value);
    return { value: resp!.value, slot: resp!.slot };
  }

  async reloadAccountData(
    client: MangoClient,
    group: Group,
  ): Promise<MangoAccount> {
    this.accountData = await client.computeAccountData(group, this);
    return this;
  }

  tokensActive(): TokenPosition[] {
    return this.tokens.filter((token) => token.isActive());
  }

  serum3Active(): Serum3Orders[] {
    return this.serum3.filter((serum3) => serum3.isActive());
  }

  perpActive(): PerpPosition[] {
    return this.perps.filter((perp) => perp.isActive());
  }

  perpOrdersActive(): PerpOo[] {
    return this.perpOpenOrders.filter(
      (oo) => oo.orderMarket !== PerpOo.OrderMarketUnset,
    );
  }

  findToken(tokenIndex: number): TokenPosition | undefined {
    return this.tokens.find((ta) => ta.tokenIndex == tokenIndex);
  }

  findSerum3Account(marketIndex: number): Serum3Orders | undefined {
    return this.serum3.find((sa) => sa.marketIndex == marketIndex);
  }

  // How to navigate
  // * if a function is returning a I80F48, then usually the return value is in native quote or native token, unless specified
  // * if a function is returning a number, then usually the return value is in ui tokens, unless specified
  // * functions try to be explicit by having native or ui in the name to better reflect the value
  // * some values might appear unexpected large or small, usually the doc contains a "note"

  /**
   *
   * @param bank
   * @returns native balance for a token, is signed
   */
  getTokenBalance(bank: Bank): I80F48 {
    const tp = this.findToken(bank.tokenIndex);
    return tp ? tp.balance(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns native deposits for a token, 0 if position has borrows
   */
  getTokenDeposits(bank: Bank): I80F48 {
    const tp = this.findToken(bank.tokenIndex);
    return tp ? tp.deposits(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns native borrows for a token, 0 if position has deposits
   */
  getTokenBorrows(bank: Bank): I80F48 {
    const tp = this.findToken(bank.tokenIndex);
    return tp ? tp.borrows(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns UI balance for a token, is signed
   */
  getTokenBalanceUi(bank: Bank): number {
    const tp = this.findToken(bank.tokenIndex);
    return tp ? tp.balanceUi(bank) : 0;
  }

  /**
   *
   * @param bank
   * @returns UI deposits for a token, 0 or more
   */
  getTokenDepositsUi(bank: Bank): number {
    const ta = this.findToken(bank.tokenIndex);
    return ta ? ta.depositsUi(bank) : 0;
  }

  /**
   *
   * @param bank
   * @returns UI borrows for a token, 0 or less
   */
  getTokenBorrowsUi(bank: Bank): number {
    const ta = this.findToken(bank.tokenIndex);
    return ta ? ta.borrowsUi(bank) : 0;
  }

  /**
   * Health, see health.rs or https://docs.mango.markets/mango-markets/health-overview
   * @param healthType
   * @returns raw health number, in native quote
   */
  getHealth(healthType: HealthType): I80F48 | undefined {
    return healthType == HealthType.init
      ? this.accountData?.initHealth
      : this.accountData?.maintHealth;
  }

  /**
   * Health ratio, which is computed so `100 * (assets-liabs)/liabs`
   * Note: health ratio is technically ∞ if liabs are 0
   * @param healthType
   * @returns health ratio, in percentage form
   */
  getHealthRatio(healthType: HealthType): I80F48 | undefined {
    return this.accountData?.healthCache.healthRatio(healthType);
  }

  /**
   * Health ratio
   * @param healthType
   * @returns health ratio, in percentage form, capped to 100
   */
  getHealthRatioUi(healthType: HealthType): number | undefined {
    const ratio = this.getHealthRatio(healthType)?.toNumber();
    if (ratio) {
      return ratio > 100 ? 100 : Math.trunc(ratio);
    } else {
      return undefined;
    }
  }

  /**
   * Sum of all the assets i.e. token deposits, borrows, total assets in spot open orders, (perps positions is todo) in terms of quote value.
   * @returns equity, in native quote
   */
  getEquity(): I80F48 | undefined {
    if (this.accountData) {
      const equity = this.accountData.equity;
      const total_equity = equity.tokens.reduce(
        (a, b) => a.add(b.value),
        ZERO_I80F48(),
      );
      return total_equity;
    }
    return undefined;
  }

  /**
   * The amount of native quote you could withdraw against your existing assets.
   * @returns collateral value, in native quote
   */
  getCollateralValue(): I80F48 | undefined {
    return this.getHealth(HealthType.init);
  }

  /**
   * Sum of all positive assets.
   * @returns assets, in native quote
   */
  getAssetsValue(healthType: HealthType): I80F48 | undefined {
    return this.accountData?.healthCache.assets(healthType);
  }

  /**
   * Sum of all negative assets.
   * @returns liabs, in native quote
   */
  getLiabsValue(healthType: HealthType): I80F48 | undefined {
    return this.accountData?.healthCache.liabs(healthType);
  }

  /**
   * The amount of given native token you can withdraw including borrows, considering all existing assets as collateral.
   * @returns amount of given native token you can borrow, considering all existing assets as collateral, in native token
   */
  getMaxWithdrawWithBorrowForToken(
    group: Group,
    mintPk: PublicKey,
  ): I80F48 | undefined {
    const tokenBank: Bank = group.getFirstBankByMint(mintPk);
    const initHealth = this.accountData?.initHealth;

    if (!initHealth) return undefined;

    // Case 1:
    // Cannot withdraw if init health is below 0
    if (initHealth.lte(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    // Deposits need special treatment since they would neither count towards liabilities
    // nor would be charged loanOriginationFeeRate when withdrawn

    const tp = this.findToken(tokenBank.tokenIndex);
    if (!tokenBank.price) return undefined;
    const existingTokenDeposits = tp ? tp.deposits(tokenBank) : ZERO_I80F48();
    let existingPositionHealthContrib = ZERO_I80F48();
    if (existingTokenDeposits.gt(ZERO_I80F48())) {
      existingPositionHealthContrib = existingTokenDeposits
        .mul(tokenBank.price)
        .imul(tokenBank.initAssetWeight);
    }

    // Case 2: token deposits have higher contribution than initHealth,
    // can withdraw without borrowing until initHealth reaches 0
    if (existingPositionHealthContrib.gt(initHealth)) {
      const withdrawAbleExistingPositionHealthContrib = initHealth;
      // console.log(`initHealth ${initHealth}`);
      // console.log(
      //   `existingPositionHealthContrib ${existingPositionHealthContrib}`,
      // );
      // console.log(
      //   `withdrawAbleExistingPositionHealthContrib ${withdrawAbleExistingPositionHealthContrib}`,
      // );
      return withdrawAbleExistingPositionHealthContrib
        .div(tokenBank.initAssetWeight)
        .div(tokenBank.price);
    }

    // Case 3: withdraw = withdraw existing deposits + borrows until initHealth reaches 0
    const initHealthWithoutExistingPosition = initHealth.sub(
      existingPositionHealthContrib,
    );
    const maxBorrowNative = initHealthWithoutExistingPosition
      .div(tokenBank.initLiabWeight)
      .div(tokenBank.price);
    const maxBorrowNativeWithoutFees = maxBorrowNative.div(
      ONE_I80F48().add(tokenBank.loanOriginationFeeRate),
    );
    // console.log(`initHealth ${initHealth}`);
    // console.log(
    //   `existingPositionHealthContrib ${existingPositionHealthContrib}`,
    // );
    // console.log(
    //   `initHealthWithoutExistingPosition ${initHealthWithoutExistingPosition}`,
    // );
    // console.log(`maxBorrowNative ${maxBorrowNative}`);
    // console.log(`maxBorrowNativeWithoutFees ${maxBorrowNativeWithoutFees}`);
    return maxBorrowNativeWithoutFees.add(existingTokenDeposits);
  }

  getMaxWithdrawWithBorrowForTokenUi(
    group: Group,
    mintPk: PublicKey,
  ): number | undefined {
    const maxWithdrawWithBorrow = this.getMaxWithdrawWithBorrowForToken(
      group,
      mintPk,
    );
    if (maxWithdrawWithBorrow) {
      return toUiDecimals(maxWithdrawWithBorrow, group.getMintDecimals(mintPk));
    } else {
      return undefined;
    }
  }

  /**
   * The max amount of given source native token you can swap to a target token.
   * note: slippageAndFeesFactor is a normalized number, <1,
   *  e.g. a slippage of 5% and some fees which are 1%, then slippageAndFeesFactor = 0.94
   *  the factor is used to compute how much target can be obtained by swapping source
   * @returns max amount of given source native token you can swap to a target token, in native token
   */
  getMaxSourceForTokenSwap(
    group: Group,
    sourceMintPk: PublicKey,
    targetMintPk: PublicKey,
    slippageAndFeesFactor: number,
  ): I80F48 | undefined {
    if (!this.accountData) return undefined;
    return this.accountData.healthCache
      .getMaxSourceForTokenSwap(
        group,
        sourceMintPk,
        targetMintPk,
        ONE_I80F48(), // target 1% health
      )
      .mul(I80F48.fromNumber(slippageAndFeesFactor));
  }

  /**
   * The max amount of given source ui token you can swap to a target token.
   * note: slippageAndFeesFactor is a normalized number, <1,
   *  e.g. a slippage of 5% and some fees which are 1%, then slippageAndFeesFactor = 0.94
   *  the factor is used to compute how much target can be obtained by swapping source
   * @returns max amount of given source ui token you can swap to a target token, in ui token
   */
  getMaxSourceUiForTokenSwap(
    group: Group,
    sourceMintPk: PublicKey,
    targetMintPk: PublicKey,
    slippageAndFeesFactor: number,
  ): number | undefined {
    const maxSource = this.getMaxSourceForTokenSwap(
      group,
      sourceMintPk,
      targetMintPk,
      slippageAndFeesFactor,
    );
    if (maxSource) {
      return toUiDecimals(maxSource, group.getMintDecimals(sourceMintPk));
    }
  }

  /**
   * Simulates new health ratio after applying tokenChanges to the token positions.
   * Note: token changes are expected in native amounts
   *
   * e.g. useful to simulate health after a potential swap.
   * Note: health ratio is technically ∞ if liabs are 0
   * @returns health ratio, in percentage form
   */
  simHealthRatioWithTokenPositionChanges(
    group: Group,
    nativeTokenChanges: {
      nativeTokenAmount: I80F48;
      mintPk: PublicKey;
    }[],
    healthType: HealthType = HealthType.init,
  ): I80F48 | undefined {
    if (!this.accountData) return undefined;
    return this.accountData.healthCache.simHealthRatioWithTokenPositionChanges(
      group,
      nativeTokenChanges,
      healthType,
    );
  }

  /**
   * Simulates new health ratio after applying tokenChanges to the token positions.
   * Note: token changes are expected in ui amounts
   *
   * e.g. useful to simulate health after a potential swap.
   * Note: health ratio is technically ∞ if liabs are 0
   * @returns health ratio, in percentage form
   */
  simHealthRatioWithTokenPositionUiChanges(
    group: Group,
    uiTokenChanges: {
      uiTokenAmount: number;
      mintPk: PublicKey;
    }[],
    healthType: HealthType = HealthType.init,
  ): number | undefined {
    const nativeTokenChanges = uiTokenChanges.map((tokenChange) => {
      return {
        nativeTokenAmount: toNative(
          tokenChange.uiTokenAmount,
          group.getMintDecimals(tokenChange.mintPk),
        ),
        mintPk: tokenChange.mintPk,
      };
    });
    return this.accountData?.healthCache
      .simHealthRatioWithTokenPositionChanges(
        group,
        nativeTokenChanges,
        healthType,
      )
      .toNumber();
  }

  public async loadSerum3OpenOrdersForMarket(
    client: MangoClient,
    group: Group,
    externalMarketPk: PublicKey,
  ): Promise<Order[]> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    );
    if (!serum3Market) {
      throw new Error(
        `Unable to find mint serum3Market for ${externalMarketPk.toString()}`,
      );
    }
    const serum3OO = this.serum3Active().find(
      (s) => s.marketIndex === serum3Market.marketIndex,
    );
    if (!serum3OO) {
      throw new Error(`No open orders account found for ${externalMarketPk}`);
    }

    const serum3MarketExternal = group.serum3MarketExternalsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const [bidsInfo, asksInfo] =
      await client.program.provider.connection.getMultipleAccountsInfo([
        serum3MarketExternal.bidsAddress,
        serum3MarketExternal.asksAddress,
      ]);
    if (!bidsInfo || !asksInfo) {
      throw new Error(
        `bids and asks ai were not fetched for ${externalMarketPk.toString()}`,
      );
    }
    const bids = Orderbook.decode(serum3MarketExternal, bidsInfo.data);
    const asks = Orderbook.decode(serum3MarketExternal, asksInfo.data);
    return [...bids, ...asks].filter((o) =>
      o.openOrdersAddress.equals(serum3OO.openOrders),
    );
  }

  /**
   *
   * @param group
   * @param serum3Market
   * @returns maximum native quote which can be traded for base token given current health
   */
  public getMaxQuoteForSerum3Bid(
    group: Group,
    serum3Market: Serum3Market,
  ): I80F48 {
    if (!this.accountData) {
      throw new Error(
        `accountData not loaded on MangoAccount, try reloading MangoAccount`,
      );
    }
    return this.accountData.healthCache.getMaxForSerum3Order(
      group,
      serum3Market,
      Serum3Side.bid,
      I80F48.fromNumber(3),
    );
  }

  public getMaxQuoteForSerum3BidUi(
    group: Group,
    externalMarketPk: PublicKey,
  ): number {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    );
    if (!serum3Market) {
      throw new Error(
        `Unable to find mint serum3Market for ${externalMarketPk.toString()}`,
      );
    }
    const nativeAmount = this.getMaxQuoteForSerum3Bid(group, serum3Market);
    return toUiDecimals(
      nativeAmount,
      group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex).mintDecimals,
    );
  }

  /**
   *
   * @param group
   * @param serum3Market
   * @returns maximum native base which can be traded for quote token given current health
   */
  public getMaxBaseForSerum3Ask(
    group: Group,
    serum3Market: Serum3Market,
  ): I80F48 {
    if (!this.accountData) {
      throw new Error(
        `accountData not loaded on MangoAccount, try reloading MangoAccount`,
      );
    }
    return this.accountData.healthCache.getMaxForSerum3Order(
      group,
      serum3Market,
      Serum3Side.ask,
      I80F48.fromNumber(3),
    );
  }

  public getMaxBaseForSerum3AskUi(
    group: Group,
    externalMarketPk: PublicKey,
  ): number {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    );
    if (!serum3Market) {
      throw new Error(
        `Unable to find mint serum3Market for ${externalMarketPk.toString()}`,
      );
    }
    const nativeAmount = this.getMaxBaseForSerum3Ask(group, serum3Market);
    return toUiDecimals(
      nativeAmount,
      group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex).mintDecimals,
    );
  }

  /**
   *
   * @param group
   * @param nativeQuoteAmount
   * @param serum3Market
   * @param healthType
   * @returns health ratio after a bid with nativeQuoteAmount is placed
   */
  simHealthRatioWithSerum3BidChanges(
    group: Group,
    nativeQuoteAmount: I80F48,
    serum3Market: Serum3Market,
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    if (!this.accountData) {
      throw new Error(
        `accountData not loaded on MangoAccount, try reloading MangoAccount`,
      );
    }
    return this.accountData.healthCache.simHealthRatioWithSerum3BidChanges(
      group,
      nativeQuoteAmount,
      serum3Market,
      healthType,
    );
  }

  simHealthRatioWithSerum3BidUiChanges(
    group: Group,
    uiQuoteAmount: number,
    externalMarketPk: PublicKey,
    healthType: HealthType = HealthType.init,
  ): number {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    );
    if (!serum3Market) {
      throw new Error(
        `Unable to find mint serum3Market for ${externalMarketPk.toString()}`,
      );
    }
    return this.simHealthRatioWithSerum3BidChanges(
      group,
      toNative(
        uiQuoteAmount,
        group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex)
          .mintDecimals,
      ),
      serum3Market,
      healthType,
    ).toNumber();
  }

  /**
   *
   * @param group
   * @param nativeBaseAmount
   * @param serum3Market
   * @param healthType
   * @returns health ratio after an ask with nativeBaseAmount is placed
   */
  simHealthRatioWithSerum3AskChanges(
    group: Group,
    nativeBaseAmount: I80F48,
    serum3Market: Serum3Market,
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    if (!this.accountData) {
      throw new Error(
        `accountData not loaded on MangoAccount, try reloading MangoAccount`,
      );
    }
    return this.accountData.healthCache.simHealthRatioWithSerum3AskChanges(
      group,
      nativeBaseAmount,
      serum3Market,
      healthType,
    );
  }

  simHealthRatioWithSerum3AskUiChanges(
    group: Group,
    uiBaseAmount: number,
    externalMarketPk: PublicKey,
    healthType: HealthType = HealthType.init,
  ): number {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    );
    if (!serum3Market) {
      throw new Error(
        `Unable to find mint serum3Market for ${externalMarketPk.toString()}`,
      );
    }
    return this.simHealthRatioWithSerum3AskChanges(
      group,
      toNative(
        uiBaseAmount,
        group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
          .mintDecimals,
      ),
      serum3Market,
      healthType,
    ).toNumber();
  }

  public async loadPerpOpenOrdersForMarket(
    client: MangoClient,
    group: Group,
    perpMarketName: string,
  ): Promise<PerpOrder[]> {
    const perpMarket = group.perpMarketsMap.get(perpMarketName);
    if (!perpMarket) {
      throw new Error(`Perp Market ${perpMarketName} not found!`);
    }
    const [bids, asks] = await Promise.all([
      perpMarket.loadBids(client),
      perpMarket.loadAsks(client),
    ]);
    return [...Array.from(bids.items()), ...Array.from(asks.items())].filter(
      (order) => order.owner.equals(this.publicKey),
    );
  }

  toString(group?: Group): string {
    let res = 'MangoAccount';
    res = res + '\n pk: ' + this.publicKey.toString();
    res = res + '\n name: ' + this.name;
    res = res + '\n owner: ' + this.owner;
    res = res + '\n delegate: ' + this.delegate;

    res =
      res +
      `\n max token slots ${this.tokens.length}, max serum3 slots ${this.serum3.length}, max perp slots ${this.perps.length}, max perp oo slots ${this.perpOpenOrders.length}`;
    res =
      this.tokensActive().length > 0
        ? res +
          '\n tokens:' +
          JSON.stringify(
            this.tokens.map((token, i) =>
              token.isActive()
                ? token.toString(group, i)
                : `index: ${i} - empty slot`,
            ),
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

    res =
      this.perpOrdersActive().length > 0
        ? res +
          '\n perps oo:' +
          JSON.stringify(this.perpOrdersActive(), null, 4)
        : res + '';

    return res;
  }
}

export class TokenPosition {
  static TokenIndexUnset = 65535;
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
    return this.tokenIndex !== TokenPosition.TokenIndexUnset;
  }

  /**
   *
   * @param bank
   * @returns native balance
   */
  public balance(bank: Bank): I80F48 {
    if (this.indexedPosition.isPos()) {
      return bank.depositIndex.mul(this.indexedPosition);
    } else {
      return bank.borrowIndex.mul(this.indexedPosition);
    }
  }

  /**
   *
   * @param bank
   * @returns native deposits, 0 if position has borrows
   */
  public deposits(bank: Bank): I80F48 {
    if (this.indexedPosition && this.indexedPosition.lt(ZERO_I80F48())) {
      return ZERO_I80F48();
    }
    return this.balance(bank);
  }

  /**
   *
   * @param bank
   * @returns native borrows, 0 if position has deposits
   */
  public borrows(bank: Bank): I80F48 {
    if (this.indexedPosition && this.indexedPosition.gt(ZERO_I80F48())) {
      return ZERO_I80F48();
    }
    return this.balance(bank).abs();
  }

  /**
   * @param bank
   * @returns UI balance, is signed
   */
  public balanceUi(bank: Bank): number {
    return nativeI80F48ToUi(this.balance(bank), bank.mintDecimals).toNumber();
  }

  /**
   * @param bank
   * @returns UI deposits, 0 if position has borrows
   */
  public depositsUi(bank: Bank): number {
    return nativeI80F48ToUi(this.deposits(bank), bank.mintDecimals).toNumber();
  }

  /**
   * @param bank
   * @returns UI borrows, 0 if position has deposits
   */
  public borrowsUi(bank: Bank): number {
    return nativeI80F48ToUi(this.borrows(bank), bank.mintDecimals).toNumber();
  }

  public toString(group?: Group, index?: number): string {
    let extra = '';
    if (group) {
      const bank: Bank = group.getFirstBankByTokenIndex(this.tokenIndex);
      if (bank) {
        const native = this.balance(bank);
        extra += ', native: ' + native.toNumber();
        extra += ', ui: ' + this.balanceUi(bank);
        extra += ', tokenName: ' + bank.name;
      }
    }

    return (
      (index !== undefined ? 'index: ' + index : '') +
      ', tokenIndex: ' +
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
  static from(dto: Serum3PositionDto): Serum3Orders {
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

export class PerpPosition {
  static PerpMarketIndexUnset = 65535;
  static from(dto: PerpPositionDto) {
    return new PerpPosition(
      dto.marketIndex,
      dto.basePositionLots.toNumber(),
      dto.quotePositionNative.val,
      dto.bidsBaseLots.toNumber(),
      dto.asksBaseLots.toNumber(),
      dto.takerBaseLots.toNumber(),
      dto.takerQuoteLots.toNumber(),
    );
  }

  constructor(
    public marketIndex: number,
    public basePositionLots: number,
    public quotePositionNative: BN,
    public bidsBaseLots: number,
    public asksBaseLots: number,
    public takerBaseLots: number,
    public takerQuoteLots: number,
  ) {}

  isActive(): boolean {
    return this.marketIndex != PerpPosition.PerpMarketIndexUnset;
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

export class PerpOo {
  static OrderMarketUnset = 65535;
  static from(dto: PerpOoDto) {
    return new PerpOo(
      dto.orderSide,
      dto.orderMarket,
      dto.clientOrderId.toNumber(),
      dto.orderId,
    );
  }

  constructor(
    public orderSide: any,
    public orderMarket: 0,
    public clientOrderId: number,
    public orderId: BN,
  ) {}
}
export class PerpOoDto {
  constructor(
    public orderSide: any,
    public orderMarket: 0,
    public clientOrderId: BN,
    public orderId: BN,
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
      HealthCache.fromDto(event.healthCache),
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
