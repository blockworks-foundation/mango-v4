import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { OpenOrders, Order, Orderbook } from '@project-serum/serum/lib/market';
import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../client';
import { SERUM3_PROGRAM_ID } from '../constants';
import {
  nativeI80F48ToUi,
  toNative,
  toUiDecimals,
  toUiDecimalsForQuote,
} from '../utils';
import { Bank, TokenIndex } from './bank';
import { Group } from './group';
import { HealthCache } from './healthCache';
import { I80F48, I80F48Dto, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { PerpMarket, PerpMarketIndex, PerpOrder, PerpOrderSide } from './perp';
import { MarketIndex, Serum3Side } from './serum3';
export class MangoAccount {
  public tokens: TokenPosition[];
  public serum3: Serum3Orders[];
  public perps: PerpPosition[];
  public perpOpenOrders: PerpOo[];
  public name: string;
  public netDeposits: BN;

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
      new Map(), // serum3OosMapByMarketIndex
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
    public serum3OosMapByMarketIndex: Map<number, OpenOrders>,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.tokens = tokens.map((dto) => TokenPosition.from(dto));
    this.serum3 = serum3.map((dto) => Serum3Orders.from(dto));
    this.perps = perps.map((dto) => PerpPosition.from(dto));
    this.perpOpenOrders = perpOpenOrders.map((dto) => PerpOo.from(dto));
    this.netDeposits = netDeposits;
  }

  async reload(client: MangoClient): Promise<MangoAccount> {
    const mangoAccount = await client.getMangoAccount(this);
    await mangoAccount.reloadAccountData(client);
    Object.assign(this, mangoAccount);
    return mangoAccount;
  }

  async reloadWithSlot(
    client: MangoClient,
  ): Promise<{ value: MangoAccount; slot: number }> {
    const resp = await client.getMangoAccountWithSlot(this.publicKey);
    await resp?.value.reloadAccountData(client);
    Object.assign(this, resp?.value);
    return { value: resp!.value, slot: resp!.slot };
  }

  async reloadAccountData(client: MangoClient): Promise<MangoAccount> {
    const serum3Active = this.serum3Active();
    const ais =
      await client.program.provider.connection.getMultipleAccountsInfo(
        serum3Active.map((serum3) => serum3.openOrders),
      );
    this.serum3OosMapByMarketIndex = new Map(
      Array.from(
        ais.map((ai, i) => {
          if (!ai) {
            throw new Error(
              `Undefined AI for open orders ${serum3Active[i].openOrders} and market ${serum3Active[i].marketIndex}!`,
            );
          }
          const oo = OpenOrders.fromAccountInfo(
            serum3Active[i].openOrders,
            ai,
            SERUM3_PROGRAM_ID[client.cluster],
          );
          return [serum3Active[i].marketIndex, oo];
        }),
      ),
    );

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

  getToken(tokenIndex: TokenIndex): TokenPosition | undefined {
    return this.tokens.find((ta) => ta.tokenIndex == tokenIndex);
  }

  getSerum3Account(marketIndex: MarketIndex): Serum3Orders | undefined {
    return this.serum3.find((sa) => sa.marketIndex == marketIndex);
  }

  getSerum3OoAccount(marketIndex: MarketIndex): OpenOrders {
    const oo: OpenOrders | undefined =
      this.serum3OosMapByMarketIndex.get(marketIndex);

    if (!oo) {
      throw new Error(
        `Open orders account not loaded for market with marketIndex ${marketIndex}!`,
      );
    }
    return oo;
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
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.balance(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns native deposits for a token, 0 if position has borrows
   */
  getTokenDeposits(bank: Bank): I80F48 {
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.deposits(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns native borrows for a token, 0 if position has deposits
   */
  getTokenBorrows(bank: Bank): I80F48 {
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.borrows(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns UI balance for a token, is signed
   */
  getTokenBalanceUi(bank: Bank): number {
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.balanceUi(bank) : 0;
  }

  /**
   *
   * @param bank
   * @returns UI deposits for a token, 0 or more
   */
  getTokenDepositsUi(bank: Bank): number {
    const ta = this.getToken(bank.tokenIndex);
    return ta ? ta.depositsUi(bank) : 0;
  }

  /**
   *
   * @param bank
   * @returns UI borrows for a token, 0 or less
   */
  getTokenBorrowsUi(bank: Bank): number {
    const ta = this.getToken(bank.tokenIndex);
    return ta ? ta.borrowsUi(bank) : 0;
  }

  /**
   * Health, see health.rs or https://docs.mango.markets/mango-markets/health-overview
   * @param healthType
   * @returns raw health number, in native quote
   */
  getHealth(group: Group, healthType: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.health(healthType);
  }

  /**
   * Health ratio, which is computed so `100 * (assets-liabs)/liabs`
   * Note: health ratio is technically ∞ if liabs are 0
   * @param healthType
   * @returns health ratio, in percentage form
   */
  getHealthRatio(group: Group, healthType: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.healthRatio(healthType);
  }

  /**
   * Health ratio
   * @param healthType
   * @returns health ratio, in percentage form, capped to 100
   */
  getHealthRatioUi(group: Group, healthType: HealthType): number | undefined {
    const ratio = this.getHealthRatio(group, healthType).toNumber();
    if (ratio) {
      return ratio > 100 ? 100 : Math.trunc(ratio);
    } else {
      return undefined;
    }
  }

  /**
   * Sum of all the assets i.e. token deposits, borrows, total assets in spot open orders, and perps positions.
   * @returns equity, in native quote
   */
  getEquity(group: Group): I80F48 {
    const tokensMap = new Map<number, I80F48>();
    for (const tp of this.tokensActive()) {
      const bank = group.getFirstBankByTokenIndex(tp.tokenIndex);
      tokensMap.set(tp.tokenIndex, tp.balance(bank).mul(bank.price));
    }

    for (const sp of this.serum3Active()) {
      const oo = this.getSerum3OoAccount(sp.marketIndex);
      const baseBank = group.getFirstBankByTokenIndex(sp.baseTokenIndex);
      tokensMap
        .get(baseBank.tokenIndex)!
        .iadd(
          I80F48.fromString(oo.baseTokenTotal.toString()).mul(baseBank.price),
        );
      const quoteBank = group.getFirstBankByTokenIndex(sp.quoteTokenIndex);
      // NOTE: referrerRebatesAccrued is not declared on oo class, but the layout
      // is aware of it
      tokensMap
        .get(baseBank.tokenIndex)!
        .iadd(
          I80F48.fromString(
            oo.quoteTokenTotal
              .add((oo as any).referrerRebatesAccrued)
              .toString(),
          ).mul(quoteBank.price),
        );
    }

    const tokenEquity = Array.from(tokensMap.values()).reduce(
      (a, b) => a.add(b),
      ZERO_I80F48(),
    );

    const perpEquity = this.perpActive().reduce(
      (a, b) =>
        a.add(b.getEquity(group.getPerpMarketByMarketIndex(b.marketIndex))),
      ZERO_I80F48(),
    );

    return tokenEquity.add(perpEquity);
  }

  /**
   * The amount of native quote you could withdraw against your existing assets.
   * @returns collateral value, in native quote
   */
  getCollateralValue(group: Group): I80F48 {
    return this.getHealth(group, HealthType.init);
  }

  /**
   * Sum of all positive assets.
   * @returns assets, in native quote
   */
  getAssetsValue(group: Group, healthType: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.assets(healthType);
  }

  /**
   * Sum of all negative assets.
   * @returns liabs, in native quote
   */
  getLiabsValue(group: Group, healthType: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.liabs(healthType);
  }

  /**
   * @returns Overall PNL, in native quote
   * PNL is defined here as spot value + serum3 open orders value + perp value - net deposits value (evaluated at native quote price at the time of the deposit/withdraw)
   * spot value + serum3 open orders value + perp value is returned by getEquity (open orders values are added to spot token values implicitly)
   */
  getPnl(group: Group): I80F48 {
    return this.getEquity(group)?.add(
      I80F48.fromI64(this.netDeposits).mul(I80F48.fromNumber(-1)),
    );
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
    const initHealth = this.getHealth(group, HealthType.init);

    if (!initHealth) return undefined;

    // Case 1:
    // Cannot withdraw if init health is below 0
    if (initHealth.lte(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    // Deposits need special treatment since they would neither count towards liabilities
    // nor would be charged loanOriginationFeeRate when withdrawn

    const tp = this.getToken(tokenBank.tokenIndex);
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
   * The max amount of given source ui token you can swap to a target token.
   *  PriceFactor is ratio between A - how many source tokens can be traded for target tokens
   *  and B - source native oracle price / target native oracle price.
   *  e.g. a slippage of 5% and some fees which are 1%, then priceFactor = 0.94
   *  the factor is used to compute how much target can be obtained by swapping source
   *  in reality, and not only relying on oracle prices, and taking in account e.g. slippage which
   *  can occur at large size
   * @returns max amount of given source ui token you can swap to a target token, in ui token
   */
  getMaxSourceUiForTokenSwap(
    group: Group,
    sourceMintPk: PublicKey,
    targetMintPk: PublicKey,
    priceFactor: number,
  ): number | undefined {
    if (sourceMintPk.equals(targetMintPk)) {
      return 0;
    }
    const hc = HealthCache.fromMangoAccount(group, this);
    const maxSource = hc.getMaxSourceForTokenSwap(
      group.getFirstBankByMint(sourceMintPk),
      group.getFirstBankByMint(targetMintPk),
      I80F48.fromNumber(2), // target 2% health
      I80F48.fromNumber(priceFactor),
    );
    maxSource.idiv(
      ONE_I80F48().add(
        group.getFirstBankByMint(sourceMintPk).loanOriginationFeeRate,
      ),
    );
    if (maxSource) {
      return toUiDecimals(maxSource, group.getMintDecimals(sourceMintPk));
    }
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
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc
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
    const serum3Market =
      group.getSerum3MarketByExternalMarket(externalMarketPk);
    const serum3OO = this.serum3Active().find(
      (s) => s.marketIndex === serum3Market.marketIndex,
    );
    if (!serum3OO) {
      throw new Error(`No open orders account found for ${externalMarketPk}`);
    }

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const [bidsInfo, asksInfo] =
      await client.program.provider.connection.getMultipleAccountsInfo([
        serum3MarketExternal.bidsAddress,
        serum3MarketExternal.asksAddress,
      ]);
    if (!bidsInfo) {
      throw new Error(
        `Undefined bidsInfo for serum3Market with externalMarket ${externalMarketPk.toString()!}`,
      );
    }
    if (!asksInfo) {
      throw new Error(
        `Undefined asksInfo for serum3Market with externalMarket ${externalMarketPk.toString()!}`,
      );
    }
    const bids = Orderbook.decode(serum3MarketExternal, bidsInfo.data);
    const asks = Orderbook.decode(serum3MarketExternal, asksInfo.data);
    return [...bids, ...asks].filter((o) =>
      o.openOrdersAddress.equals(serum3OO.openOrders),
    );
  }

  /**
   * @param group
   * @param externalMarketPk
   * @returns maximum ui quote which can be traded for base token given current health
   */
  public getMaxQuoteForSerum3BidUi(
    group: Group,
    externalMarketPk: PublicKey,
  ): number {
    const serum3Market =
      group.getSerum3MarketByExternalMarket(externalMarketPk);
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );
    const hc = HealthCache.fromMangoAccount(group, this);
    let nativeAmount = hc.getMaxSerum3OrderForHealthRatio(
      baseBank,
      quoteBank,
      serum3Market,
      Serum3Side.bid,
      I80F48.fromNumber(2),
    );
    // If its a bid then the reserved fund and potential loan is in base
    // also keep some buffer for fees, use taker fees for worst case simulation.
    nativeAmount = nativeAmount
      .div(quoteBank.price)
      .div(ONE_I80F48().add(baseBank.loanOriginationFeeRate))
      .div(ONE_I80F48().add(I80F48.fromNumber(group.getSerum3FeeRates(false))));
    return toUiDecimals(
      nativeAmount,
      group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex).mintDecimals,
    );
  }

  /**
   * @param group
   * @param externalMarketPk
   * @returns maximum ui base which can be traded for quote token given current health
   */
  public getMaxBaseForSerum3AskUi(
    group: Group,
    externalMarketPk: PublicKey,
  ): number {
    const serum3Market =
      group.getSerum3MarketByExternalMarket(externalMarketPk);
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );
    const hc = HealthCache.fromMangoAccount(group, this);
    let nativeAmount = hc.getMaxSerum3OrderForHealthRatio(
      baseBank,
      quoteBank,
      serum3Market,
      Serum3Side.ask,
      I80F48.fromNumber(2),
    );
    // If its a ask then the reserved fund and potential loan is in base
    // also keep some buffer for fees, use taker fees for worst case simulation.
    nativeAmount = nativeAmount
      .div(baseBank.price)
      .div(ONE_I80F48().add(baseBank.loanOriginationFeeRate))
      .div(ONE_I80F48().add(I80F48.fromNumber(group.getSerum3FeeRates(false))));
    return toUiDecimals(
      nativeAmount,
      group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex).mintDecimals,
    );
  }

  /**
   *
   * @param group
   * @param uiQuoteAmount
   * @param externalMarketPk
   * @param healthType
   * @returns health ratio after a bid with uiQuoteAmount is placed
   */
  public simHealthRatioWithSerum3BidUiChanges(
    group: Group,
    uiQuoteAmount: number,
    externalMarketPk: PublicKey,
    healthType: HealthType = HealthType.init,
  ): number {
    const serum3Market =
      group.getSerum3MarketByExternalMarket(externalMarketPk);
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc
      .simHealthRatioWithSerum3BidChanges(
        baseBank,
        quoteBank,
        toNative(
          uiQuoteAmount,
          group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex)
            .mintDecimals,
        ),
        serum3Market,
        healthType,
      )
      .toNumber();
  }

  /**
   *
   * @param group
   * @param uiBaseAmount
   * @param externalMarketPk
   * @param healthType
   * @returns health ratio after an ask with uiBaseAmount is placed
   */
  public simHealthRatioWithSerum3AskUiChanges(
    group: Group,
    uiBaseAmount: number,
    externalMarketPk: PublicKey,
    healthType: HealthType = HealthType.init,
  ): number {
    const serum3Market =
      group.getSerum3MarketByExternalMarket(externalMarketPk);
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc
      .simHealthRatioWithSerum3AskChanges(
        baseBank,
        quoteBank,
        toNative(
          uiBaseAmount,
          group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
            .mintDecimals,
        ),
        serum3Market,
        healthType,
      )
      .toNumber();
  }

  /**
   *
   * @param group
   * @param perpMarketName
   * @param uiPrice ui price at which bid would be placed at
   * @returns max ui quote bid
   */
  public getMaxQuoteForPerpBidUi(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    uiPrice: number,
  ): number {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const hc = HealthCache.fromMangoAccount(group, this);
    const baseLots = hc.getMaxPerpForHealthRatio(
      perpMarket,
      PerpOrderSide.bid,
      I80F48.fromNumber(2),
      group.toNativePrice(uiPrice, perpMarket.baseDecimals),
    );
    const nativeBase = baseLots.mul(
      I80F48.fromString(perpMarket.baseLotSize.toString()),
    );
    const nativeQuote = nativeBase.mul(perpMarket.price);
    return toUiDecimalsForQuote(nativeQuote.toNumber());
  }

  /**
   *
   * @param group
   * @param perpMarketName
   * @param uiPrice ui price at which ask would be placed at
   * @returns max ui base ask
   */
  public getMaxBaseForPerpAskUi(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    uiPrice: number,
  ): number {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const hc = HealthCache.fromMangoAccount(group, this);
    const baseLots = hc.getMaxPerpForHealthRatio(
      perpMarket,
      PerpOrderSide.ask,
      I80F48.fromNumber(2),
      group.toNativePrice(uiPrice, perpMarket.baseDecimals),
    );
    return perpMarket.baseLotsToUi(new BN(baseLots.toString()));
  }

  public async loadPerpOpenOrdersForMarket(
    client: MangoClient,
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<PerpOrder[]> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
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
  static from(dto: TokenPositionDto): TokenPosition {
    return new TokenPosition(
      I80F48.from(dto.indexedPosition),
      dto.tokenIndex as TokenIndex,
      dto.inUseCount,
    );
  }

  constructor(
    public indexedPosition: I80F48,
    public tokenIndex: TokenIndex,
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
      dto.marketIndex as MarketIndex,
      dto.baseTokenIndex as TokenIndex,
      dto.quoteTokenIndex as TokenIndex,
    );
  }

  constructor(
    public openOrders: PublicKey,
    public marketIndex: MarketIndex,
    public baseTokenIndex: TokenIndex,
    public quoteTokenIndex: TokenIndex,
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
  static from(dto: PerpPositionDto): PerpPosition {
    return new PerpPosition(
      dto.marketIndex as PerpMarketIndex,
      dto.basePositionLots.toNumber(),
      I80F48.from(dto.quotePositionNative),
      dto.bidsBaseLots.toNumber(),
      dto.asksBaseLots.toNumber(),
      dto.takerBaseLots.toNumber(),
      dto.takerQuoteLots.toNumber(),
      I80F48.from(dto.longSettledFunding),
      I80F48.from(dto.shortSettledFunding),
    );
  }

  constructor(
    public marketIndex: PerpMarketIndex,
    public basePositionLots: number,
    public quotePositionNative: I80F48,
    public bidsBaseLots: number,
    public asksBaseLots: number,
    public takerBaseLots: number,
    public takerQuoteLots: number,
    public longSettledFunding: I80F48,
    public shortSettledFunding: I80F48,
  ) {}

  isActive(): boolean {
    return this.marketIndex != PerpPosition.PerpMarketIndexUnset;
  }

  public unsettledFunding(perpMarket: PerpMarket): I80F48 {
    if (this.basePositionLots > 0) {
      return perpMarket.longFunding
        .sub(this.longSettledFunding)
        .mul(I80F48.fromString(this.basePositionLots.toString()));
    } else if (this.basePositionLots < 0) {
      return perpMarket.shortFunding
        .sub(this.shortSettledFunding)
        .mul(I80F48.fromString(this.basePositionLots.toString()));
    }
    return ZERO_I80F48();
  }

  public getEquity(perpMarket: PerpMarket): I80F48 {
    const lotsToQuote = I80F48.fromString(
      perpMarket.baseLotSize.toString(),
    ).mul(perpMarket.price);

    const baseLots = I80F48.fromNumber(
      this.basePositionLots + this.takerBaseLots,
    );

    const unsettledFunding = this.unsettledFunding(perpMarket);
    const takerQuote = I80F48.fromString(
      new BN(this.takerQuoteLots).mul(perpMarket.quoteLotSize).toString(),
    );
    const quoteCurrent = I80F48.fromString(this.quotePositionNative.toString())
      .sub(unsettledFunding)
      .add(takerQuote);

    return baseLots.mul(lotsToQuote).add(quoteCurrent);
  }

  public hasOpenOrders(): boolean {
    return (
      this.asksBaseLots != 0 ||
      this.bidsBaseLots != 0 ||
      this.takerBaseLots != 0 ||
      this.takerQuoteLots != 0
    );
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
    public longSettledFunding: I80F48Dto,
    public shortSettledFunding: I80F48Dto,
  ) {}
}

export class PerpOo {
  static OrderMarketUnset = 65535;
  static from(dto: PerpOoDto): PerpOo {
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
