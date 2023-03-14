import { AnchorProvider, BN } from '@coral-xyz/anchor';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { OpenOrders, Order, Orderbook } from '@project-serum/serum/lib/market';
import { AccountInfo, PublicKey, TransactionSignature } from '@solana/web3.js';
import { MangoClient } from '../client';
import { OPENBOOK_PROGRAM_ID, RUST_I64_MAX, RUST_I64_MIN } from '../constants';
import { I80F48, I80F48Dto, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { toNativeI80F48, toUiDecimals, toUiDecimalsForQuote } from '../utils';
import { Bank, TokenIndex } from './bank';
import { Group } from './group';
import { HealthCache } from './healthCache';
import { PerpMarket, PerpMarketIndex, PerpOrder, PerpOrderSide } from './perp';
import { MarketIndex, Serum3Side } from './serum3';
export class MangoAccount {
  public name: string;
  public tokens: TokenPosition[];
  public serum3: Serum3Orders[];
  public perps: PerpPosition[];
  public perpOpenOrders: PerpOo[];

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      owner: PublicKey;
      name: number[];
      delegate: PublicKey;
      accountNum: number;
      beingLiquidated: number;
      inHealthRegion: number;
      netDeposits: BN;
      perpSpotTransfers: BN;
      healthRegionBeginInitHealth: BN;
      frozenUntil: BN;
      buybackFeesAccruedCurrent: BN;
      buybackFeesAccruedPrevious: BN;
      buybackFeesExpiryTimestamp: BN;
      headerVersion: number;
      tokens: unknown;
      serum3: unknown;
      perps: unknown;
      perpOpenOrders: unknown;
    },
  ): MangoAccount {
    return new MangoAccount(
      publicKey,
      obj.group,
      obj.owner,
      obj.name,
      obj.delegate,
      obj.accountNum,
      obj.beingLiquidated == 1,
      obj.inHealthRegion == 1,
      obj.netDeposits,
      obj.perpSpotTransfers,
      obj.healthRegionBeginInitHealth,
      obj.frozenUntil,
      obj.buybackFeesAccruedCurrent,
      obj.buybackFeesAccruedPrevious,
      obj.buybackFeesExpiryTimestamp,
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
    public accountNum: number,
    public beingLiquidated: boolean,
    public inHealthRegion: boolean,
    public netDeposits: BN,
    public perpSpotTransfers: BN,
    public healthRegionBeginInitHealth: BN,
    public frozenUntil: BN,
    public buybackFeesAccruedCurrent: BN,
    public buybackFeesAccruedPrevious: BN,
    public buybackFeesExpiryTimestamp: BN,
    public headerVersion: number,
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
  }

  public async reload(client: MangoClient): Promise<MangoAccount> {
    const mangoAccount = await client.getMangoAccount(this);
    await mangoAccount.reloadSerum3OpenOrders(client);
    Object.assign(this, mangoAccount);
    return mangoAccount;
  }

  public async reloadWithSlot(
    client: MangoClient,
  ): Promise<{ value: MangoAccount; slot: number }> {
    const resp = await client.getMangoAccountWithSlot(this.publicKey);
    await resp?.value.reloadSerum3OpenOrders(client);
    Object.assign(this, resp?.value);
    return { value: resp!.value, slot: resp!.slot };
  }

  async reloadSerum3OpenOrders(client: MangoClient): Promise<MangoAccount> {
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
            OPENBOOK_PROGRAM_ID[client.cluster],
          );
          return [serum3Active[i].marketIndex, oo];
        }),
      ),
    );

    return this;
  }

  public isDelegate(client: MangoClient): boolean {
    return this.delegate.equals(
      (client.program.provider as AnchorProvider).wallet.publicKey,
    );
  }

  public isOperational(): boolean {
    return this.frozenUntil.lt(new BN(Date.now() / 1000));
  }

  public tokensActive(): TokenPosition[] {
    return this.tokens.filter((token) => token.isActive());
  }

  public serum3Active(): Serum3Orders[] {
    return this.serum3.filter((serum3) => serum3.isActive());
  }

  public perpPositionExistsForMarket(perpMarket: PerpMarket): boolean {
    return this.perps.some(
      (pp) => pp.isActive() && pp.marketIndex == perpMarket.perpMarketIndex,
    );
  }

  public perpOrderExistsForMarket(perpMarket: PerpMarket): boolean {
    return this.perpOpenOrders.some(
      (poo) => poo.isActive() && poo.orderMarket == perpMarket.perpMarketIndex,
    );
  }

  public perpActive(): PerpPosition[] {
    return this.perps.filter((perp) => perp.isActive());
  }

  public perpOrdersActive(): PerpOo[] {
    return this.perpOpenOrders.filter(
      (oo) => oo.orderMarket !== PerpOo.OrderMarketUnset,
    );
  }

  public getToken(tokenIndex: TokenIndex): TokenPosition | undefined {
    return this.tokens.find((ta) => ta.tokenIndex == tokenIndex);
  }

  public getSerum3Account(marketIndex: MarketIndex): Serum3Orders | undefined {
    return this.serum3.find((sa) => sa.marketIndex == marketIndex);
  }

  public getPerpPosition(
    perpMarketIndex: PerpMarketIndex,
  ): PerpPosition | undefined {
    return this.perps.find((pp) => pp.marketIndex == perpMarketIndex);
  }

  public getPerpPositionUi(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    useEventQueue?: boolean,
  ): number {
    const pp = this.perps.find((pp) => pp.marketIndex == perpMarketIndex);
    if (!pp) {
      throw new Error(`No position found for PerpMarket ${perpMarketIndex}!`);
    }
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return pp.getBasePositionUi(perpMarket, useEventQueue);
  }

  public getSerum3OoAccount(marketIndex: MarketIndex): OpenOrders {
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
  public getTokenBalance(bank: Bank): I80F48 {
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.balance(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns native deposits for a token, 0 if position has borrows
   */
  public getTokenDeposits(bank: Bank): I80F48 {
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.deposits(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns native borrows for a token, 0 if position has deposits
   */
  public getTokenBorrows(bank: Bank): I80F48 {
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.borrows(bank) : ZERO_I80F48();
  }

  /**
   *
   * @param bank
   * @returns UI balance for a token, is signed
   */
  public getTokenBalanceUi(bank: Bank): number {
    const tp = this.getToken(bank.tokenIndex);
    return tp ? tp.balanceUi(bank) : 0;
  }

  /**
   *
   * @param bank
   * @returns UI deposits for a token, 0 or more
   */
  public getTokenDepositsUi(bank: Bank): number {
    const ta = this.getToken(bank.tokenIndex);
    return ta ? ta.depositsUi(bank) : 0;
  }

  /**
   *
   * @param bank
   * @returns UI borrows for a token, 0 or less
   */
  public getTokenBorrowsUi(bank: Bank): number {
    const ta = this.getToken(bank.tokenIndex);
    return ta ? ta.borrowsUi(bank) : 0;
  }

  /**
   * Health, see health.rs or https://docs.mango.markets/mango-markets/health-overview
   * @param healthType
   * @returns raw health number, in native quote
   */
  public getHealth(group: Group, healthType: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.health(healthType);
  }

  public getPerpSettleHealth(group: Group): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.perpSettleHealth();
  }

  /**
   * Health ratio, which is computed so `100 * (assets-liabs)/liabs`
   * Note: health ratio is technically ∞ if liabs are 0
   * @param healthType
   * @returns health ratio, in percentage form
   */
  public getHealthRatio(group: Group, healthType: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.healthRatio(healthType);
  }

  /**
   * Health ratio
   * @param healthType
   * @returns health ratio, in percentage form, capped to 100
   */
  public getHealthRatioUi(group: Group, healthType: HealthType): number {
    const ratio = this.getHealthRatio(group, healthType).toNumber();
    return ratio > 100 ? 100 : Math.trunc(ratio);
  }

  /**
   * Sum of all the assets i.e. token deposits, borrows, total assets in spot open orders, and perps positions.
   * @returns equity, in native quote
   */
  public getEquity(group: Group): I80F48 {
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
        .iadd(I80F48.fromI64(oo.baseTokenTotal).mul(baseBank.price));
      const quoteBank = group.getFirstBankByTokenIndex(sp.quoteTokenIndex);
      // NOTE: referrerRebatesAccrued is not declared on oo class, but the layout
      // is aware of it
      tokensMap
        .get(baseBank.tokenIndex)!
        .iadd(
          I80F48.fromI64(
            oo.quoteTokenTotal.add((oo as any).referrerRebatesAccrued),
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
  public getCollateralValue(group: Group): I80F48 {
    return this.getHealth(group, HealthType.init);
  }

  /**
   * Sum of all positive assets.
   * @returns assets, in native quote
   */
  public getAssetsValue(group: Group, healthType?: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.assets(healthType);
  }

  /**
   * Sum of all negative assets.
   * @returns liabs, in native quote
   */
  public getLiabsValue(group: Group, healthType?: HealthType): I80F48 {
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc.liabs(healthType);
  }

  /**
   * @returns Overall PNL, in native quote
   * PNL is defined here as spot value + serum3 open orders value + perp value - net deposits value (evaluated at native quote price at the time of the deposit/withdraw)
   * spot value + serum3 open orders value + perp value is returned by getEquity (open orders values are added to spot token values implicitly)
   */
  public getPnl(group: Group): I80F48 {
    return this.getEquity(group)?.add(
      I80F48.fromI64(this.netDeposits).mul(I80F48.fromNumber(-1)),
    );
  }

  /**
   * @returns token cumulative interest, in native token units. Sum of deposit and borrow interest.
   * Caveat: This will only return cumulative interest since the tokenPosition was last opened.
   * If the tokenPosition was closed and reopened multiple times it is necessary to add this result to
   * cumulative interest at each of the prior tokenPosition closings (from mango API) to get the all time
   * cumulative interest.
   */
  getCumulativeInterest(bank: Bank): number {
    const token = this.getToken(bank.tokenIndex);

    if (token === undefined) {
      // tokenPosition does not exist on mangoAccount so no cumulative interest
      return 0;
    } else {
      if (token.indexedPosition.isPos()) {
        const interest = bank.depositIndex
          .sub(token.previousIndex)
          .mul(token.indexedPosition)
          .toNumber();
        return (
          interest +
          token.cumulativeDepositInterest +
          token.cumulativeBorrowInterest
        );
      } else {
        const interest = bank.borrowIndex
          .sub(token.previousIndex)
          .mul(token.indexedPosition)
          .toNumber();
        return (
          interest +
          token.cumulativeDepositInterest +
          token.cumulativeBorrowInterest
        );
      }
    }
  }

  /**
   * The amount of given native token you can withdraw including borrows, considering all existing assets as collateral.
   * @returns amount of given native token you can borrow, considering all existing assets as collateral, in native token
   *
   * TODO: take into account net_borrow_limit and min_vault_to_deposits_ratio
   */
  public getMaxWithdrawWithBorrowForToken(
    group: Group,
    mintPk: PublicKey,
  ): I80F48 {
    const tokenBank: Bank = group.getFirstBankByMint(mintPk);
    const initHealth = this.getHealth(group, HealthType.init);

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
    let maxBorrowNative = initHealthWithoutExistingPosition
      .div(tokenBank.initLiabWeight)
      .div(tokenBank.price);

    // Cap maxBorrow to maintain minVaultToDepositsRatio on the bank
    const vaultAmount = group.vaultAmountsMap.get(tokenBank.vault.toBase58());
    if (!vaultAmount) {
      throw new Error(
        `No vault amount found for ${tokenBank.name} vault ${tokenBank.vault}!`,
      );
    }
    const vaultAmountAfterWithdrawingDeposits = I80F48.fromU64(vaultAmount).sub(
      existingTokenDeposits,
    );
    const expectedVaultMinAmount = tokenBank
      .nativeDeposits()
      .mul(I80F48.fromNumber(tokenBank.minVaultToDepositsRatio));
    if (vaultAmountAfterWithdrawingDeposits.gt(expectedVaultMinAmount)) {
      maxBorrowNative = maxBorrowNative.min(
        vaultAmountAfterWithdrawingDeposits.sub(expectedVaultMinAmount),
      );
    }

    const maxBorrowNativeWithoutFees = maxBorrowNative.div(
      ONE_I80F48().add(tokenBank.loanOriginationFeeRate),
    );

    return maxBorrowNativeWithoutFees.add(existingTokenDeposits);
  }

  public getMaxWithdrawWithBorrowForTokenUi(
    group: Group,
    mintPk: PublicKey,
  ): number {
    const maxWithdrawWithBorrow = this.getMaxWithdrawWithBorrowForToken(
      group,
      mintPk,
    );
    return toUiDecimals(maxWithdrawWithBorrow, group.getMintDecimals(mintPk));
  }

  /**
   * The max amount of given source ui token you can swap to a target token.
   * @returns max amount of given source ui token you can swap to a target token, in ui token
   */
  getMaxSourceUiForTokenSwap(
    group: Group,
    sourceMintPk: PublicKey,
    targetMintPk: PublicKey,
    slippageAndFeesFactor = 1,
  ): number {
    if (sourceMintPk.equals(targetMintPk)) {
      return 0;
    }
    const sourceBank = group.getFirstBankByMint(sourceMintPk);
    const targetBank = group.getFirstBankByMint(targetMintPk);
    const hc = HealthCache.fromMangoAccount(group, this);
    let maxSource = hc.getMaxSwapSource(
      sourceBank,
      targetBank,
      I80F48.fromNumber(
        slippageAndFeesFactor *
          ((sourceBank.uiPrice / targetBank.uiPrice) *
            Math.pow(10, targetBank.mintDecimals - sourceBank.mintDecimals)),
      ),
    );
    const sourceBalance = this.getTokenBalance(sourceBank);
    if (maxSource.gt(sourceBalance)) {
      const sourceBorrow = maxSource.sub(sourceBalance);
      maxSource = sourceBalance.add(
        sourceBorrow.div(ONE_I80F48().add(sourceBank.loanOriginationFeeRate)),
      );
    }
    return toUiDecimals(maxSource, group.getMintDecimals(sourceMintPk));
  }

  /**
   * Simulates new health ratio after applying tokenChanges to the token positions.
   * Note: token changes are expected in ui amounts
   *
   * e.g. useful to simulate health after a potential swap.
   * Note: health ratio is technically ∞ if liabs are 0
   * @returns health ratio, in percentage form
   */
  public simHealthRatioWithTokenPositionUiChanges(
    group: Group,
    uiTokenChanges: {
      uiTokenAmount: number;
      mintPk: PublicKey;
    }[],
    healthType: HealthType = HealthType.init,
  ): number {
    const nativeTokenChanges = uiTokenChanges.map((tokenChange) => {
      return {
        nativeTokenAmount: toNativeI80F48(
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

  public async loadSerum3OpenOrdersAccounts(
    client: MangoClient,
  ): Promise<OpenOrders[]> {
    const openOrderPks = this.serum3Active().map((s) => s.openOrders);
    if (!openOrderPks.length) return [];
    const response =
      await client.program.provider.connection.getMultipleAccountsInfo(
        openOrderPks,
      );
    const accounts = response.filter((a): a is AccountInfo<Buffer> =>
      Boolean(a),
    );

    return accounts.map((acc, index) => {
      return OpenOrders.fromAccountInfo(
        this.serum3[index].openOrders,
        acc,
        OPENBOOK_PROGRAM_ID[client.cluster],
      );
    });
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
   * TODO REWORK, know to break in binary search, also make work for limit orders
   *
   * @param group
   * @param externalMarketPk
   * @returns maximum ui quote which can be traded at oracle price for base token given current health
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
    const nativeAmount = hc.getMaxSerum3OrderForHealthRatio(
      baseBank,
      quoteBank,
      serum3Market,
      Serum3Side.bid,
      I80F48.fromNumber(2),
    );
    let quoteAmount = nativeAmount.div(quoteBank.price);
    // If its a bid then the reserved fund and potential loan is in base
    // also keep some buffer for fees, use taker fees for worst case simulation.
    const quoteBalance = this.getTokenBalance(quoteBank);
    if (quoteAmount.gt(quoteBalance)) {
      const quoteBorrow = quoteAmount.sub(quoteBalance);
      quoteAmount = quoteBalance.add(
        quoteBorrow.div(ONE_I80F48().add(quoteBank.loanOriginationFeeRate)),
      );
    }
    quoteAmount = quoteAmount.div(
      ONE_I80F48().add(I80F48.fromNumber(serum3Market.getFeeRates(true))),
    );
    return toUiDecimals(nativeAmount, quoteBank.mintDecimals);
  }

  /**
   * TODO REWORK, know to break in binary search, also make work for limit orders
   * @param group
   * @param externalMarketPk
   * @returns maximum ui base which can be traded at oracle price for quote token given current health
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
    const nativeAmount = hc.getMaxSerum3OrderForHealthRatio(
      baseBank,
      quoteBank,
      serum3Market,
      Serum3Side.ask,
      I80F48.fromNumber(2),
    );
    let baseAmount = nativeAmount.div(baseBank.price);
    // If its a ask then the reserved fund and potential loan is in base
    // also keep some buffer for fees, use taker fees for worst case simulation.
    const baseBalance = this.getTokenBalance(baseBank);
    if (baseAmount.gt(baseBalance)) {
      const baseBorrow = baseAmount.sub(baseBalance);
      baseAmount = baseBalance.add(
        baseBorrow.div(ONE_I80F48().add(baseBank.loanOriginationFeeRate)),
      );
    }
    baseAmount = baseAmount.div(
      ONE_I80F48().add(I80F48.fromNumber(serum3Market.getFeeRates(true))),
    );
    return toUiDecimals(baseAmount, baseBank.mintDecimals);
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
        toNativeI80F48(
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
        toNativeI80F48(
          uiBaseAmount,
          group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
            .mintDecimals,
        ),
        serum3Market,
        healthType,
      )
      .toNumber();
  }

  // TODO: don't send a settle instruction if there's nothing to settle
  public async serum3SettleFundsForAllMarkets(
    client: MangoClient,
    group: Group,
  ): Promise<TransactionSignature[]> {
    // Future: collect ixs, batch them, and send them in fewer txs
    return await Promise.all(
      this.serum3Active().map((s) => {
        const serum3Market = group.getSerum3MarketByMarketIndex(s.marketIndex);
        return client.serum3SettleFunds(
          group,
          this,
          serum3Market.serumMarketExternal,
        );
      }),
    );
  }

  // TODO: cancel until all are cancelled
  public async serum3CancelAllOrdersForAllMarkets(
    client: MangoClient,
    group: Group,
  ): Promise<TransactionSignature[]> {
    // Future: collect ixs, batch them, and send them in in fewer txs
    return await Promise.all(
      this.serum3Active().map((s) => {
        const serum3Market = group.getSerum3MarketByMarketIndex(s.marketIndex);
        return client.serum3CancelAllOrders(
          group,
          this,
          serum3Market.serumMarketExternal,
        );
      }),
    );
  }

  /**
   * TODO: also think about limit orders
   *
   * The max ui quote you can place a market/ioc bid on the market,
   * price is the ui price at which you think the order would materialiase.
   * @param group
   * @param perpMarketName
   * @returns maximum ui quote which can be traded at oracle price for quote token given current health
   */
  public getMaxQuoteForPerpBidUi(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): number {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const hc = HealthCache.fromMangoAccount(group, this);
    const baseLots = hc.getMaxPerpForHealthRatio(
      perpMarket,
      perpMarket.price,
      PerpOrderSide.bid,
      I80F48.fromNumber(2),
    );
    const nativeBase = baseLots.mul(I80F48.fromI64(perpMarket.baseLotSize));
    const nativeQuote = nativeBase.mul(perpMarket.price);
    return toUiDecimalsForQuote(nativeQuote);
  }

  /**
   * TODO: also think about limit orders
   *
   * The max ui base you can place a market/ioc ask on the market,
   * price is the ui price at which you think the order would materialiase.
   * @param group
   * @param perpMarketName
   * @param uiPrice ui price at which ask would be placed at
   * @returns max ui base ask
   */
  public getMaxBaseForPerpAskUi(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): number {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const hc = HealthCache.fromMangoAccount(group, this);
    const baseLots = hc.getMaxPerpForHealthRatio(
      perpMarket,
      perpMarket.price,
      PerpOrderSide.ask,
      I80F48.fromNumber(2),
    );
    return perpMarket.baseLotsToUi(new BN(baseLots.toString()));
  }

  public simHealthRatioWithPerpBidUiChanges(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    size: number,
  ): number {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const pp = this.getPerpPosition(perpMarket.perpMarketIndex);
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc
      .simHealthRatioWithPerpOrderChanges(
        perpMarket,
        pp
          ? pp
          : PerpPosition.emptyFromPerpMarketIndex(perpMarket.perpMarketIndex),
        PerpOrderSide.bid,
        perpMarket.uiBaseToLots(size),
        perpMarket.price,
        HealthType.init,
      )
      .toNumber();
  }

  public simHealthRatioWithPerpAskUiChanges(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    size: number,
  ): number {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const pp = this.getPerpPosition(perpMarket.perpMarketIndex);
    const hc = HealthCache.fromMangoAccount(group, this);
    return hc
      .simHealthRatioWithPerpOrderChanges(
        perpMarket,
        pp
          ? pp
          : PerpPosition.emptyFromPerpMarketIndex(perpMarket.perpMarketIndex),
        PerpOrderSide.ask,
        perpMarket.uiBaseToLots(size),
        perpMarket.price,
        HealthType.init,
      )
      .toNumber();
  }

  public async loadPerpOpenOrdersForMarket(
    client: MangoClient,
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    forceReload?: boolean,
  ): Promise<PerpOrder[]> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const [bids, asks] = await Promise.all([
      perpMarket.loadBids(client, forceReload),
      perpMarket.loadAsks(client, forceReload),
    ]);

    return [...bids.items(), ...asks.items()].filter((order) =>
      order.owner.equals(this.publicKey),
    );
  }

  public getBuybackFeesAccrued(): BN {
    return this.buybackFeesAccruedCurrent.add(this.buybackFeesAccruedPrevious);
  }

  public getBuybackFeesAccruedUi(): number {
    return toUiDecimalsForQuote(this.getBuybackFeesAccrued());
  }

  public getMaxFeesBuyback(group: Group): BN {
    const mngoBalanceValueWithBonus = new BN(
      this.getTokenBalance(group.getFirstBankForMngo())
        .mul(group.getFirstBankForMngo().price)
        .mul(I80F48.fromNumber(group.buybackFeesMngoBonusFactor))
        .floor()
        .toNumber(),
    );
    return BN.max(
      BN.min(this.getBuybackFeesAccrued(), mngoBalanceValueWithBonus),
      new BN(0),
    );
  }

  public getMaxFeesBuybackUi(group: Group): number {
    return toUiDecimalsForQuote(this.getMaxFeesBuyback(group));
  }

  toString(group?: Group, onlyTokens = false): string {
    let res = 'MangoAccount';
    res = res + '\n pk: ' + this.publicKey.toString();
    res = res + '\n name: ' + this.name;
    res = res + '\n accountNum: ' + this.accountNum;
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
            this.tokens
              .filter((token, i) => token.isActive())
              .map((token, i) => token.toString(group, i)),
            null,
            4,
          )
        : res + '';

    if (onlyTokens) {
      return res;
    }

    res =
      this.serum3Active().length > 0
        ? res + '\n serum:' + JSON.stringify(this.serum3Active(), null, 4)
        : res + '';

    res =
      this.perpActive().length > 0
        ? res +
          '\n perps:' +
          JSON.stringify(
            this.perpActive().map((p) =>
              p.toString(group?.getPerpMarketByMarketIndex(p.marketIndex)),
            ),
            null,
            4,
          )
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
      I80F48.from(dto.previousIndex),
      dto.cumulativeDepositInterest,
      dto.cumulativeBorrowInterest,
    );
  }

  constructor(
    public indexedPosition: I80F48,
    public tokenIndex: TokenIndex,
    public inUseCount: number,
    public previousIndex: I80F48,
    public cumulativeDepositInterest: number,
    public cumulativeBorrowInterest: number,
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
    return toUiDecimals(this.balance(bank), bank.mintDecimals);
  }

  /**
   * @param bank
   * @returns UI deposits, 0 if position has borrows
   */
  public depositsUi(bank: Bank): number {
    return toUiDecimals(this.deposits(bank), bank.mintDecimals);
  }

  /**
   * @param bank
   * @returns UI borrows, 0 if position has deposits
   */
  public borrowsUi(bank: Bank): number {
    return toUiDecimals(this.borrows(bank), bank.mintDecimals);
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
    public previousIndex: I80F48Dto,
    public cumulativeDepositInterest: number,
    public cumulativeBorrowInterest: number,
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
    public baseBorrowsWithoutFee: BN,
    public quoteBorrowsWithoutFee: BN,
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
      dto.settlePnlLimitWindow,
      dto.settlePnlLimitSettledInCurrentWindowNative,
      dto.basePositionLots,
      I80F48.from(dto.quotePositionNative),
      dto.quoteRunningNative,
      I80F48.from(dto.longSettledFunding),
      I80F48.from(dto.shortSettledFunding),
      dto.bidsBaseLots,
      dto.asksBaseLots,
      dto.takerBaseLots,
      dto.takerQuoteLots,
      dto.cumulativeLongFunding,
      dto.cumulativeShortFunding,
      dto.makerVolume,
      dto.takerVolume,
      dto.perpSpotTransfers,
      dto.avgEntryPricePerBaseLot,
      I80F48.from(dto.realizedTradePnlNative),
      I80F48.from(dto.realizedOtherPnlNative),
      dto.settlePnlLimitRealizedTrade,
      I80F48.from(dto.realizedPnlForPositionNative),
    );
  }

  static emptyFromPerpMarketIndex(
    perpMarketIndex: PerpMarketIndex,
  ): PerpPosition {
    return new PerpPosition(
      perpMarketIndex,
      0,
      new BN(0),
      new BN(0),
      ZERO_I80F48(),
      new BN(0),
      ZERO_I80F48(),
      ZERO_I80F48(),
      new BN(0),
      new BN(0),
      new BN(0),
      new BN(0),
      0,
      0,
      new BN(0),
      new BN(0),
      new BN(0),
      0,
      ZERO_I80F48(),
      ZERO_I80F48(),
      new BN(0),
      ZERO_I80F48(),
    );
  }

  constructor(
    public marketIndex: PerpMarketIndex,
    public settlePnlLimitWindow: number,
    public settlePnlLimitSettledInCurrentWindowNative: BN,
    public basePositionLots: BN,
    public quotePositionNative: I80F48,
    public quoteRunningNative: BN,
    public longSettledFunding: I80F48,
    public shortSettledFunding: I80F48,
    public bidsBaseLots: BN,
    public asksBaseLots: BN,
    public takerBaseLots: BN,
    public takerQuoteLots: BN,
    public cumulativeLongFunding: number,
    public cumulativeShortFunding: number,
    public makerVolume: BN,
    public takerVolume: BN,
    public perpSpotTransfers: BN,
    public avgEntryPricePerBaseLot: number,
    public realizedTradePnlNative: I80F48,
    public realizedOtherPnlNative: I80F48,
    public settlePnlLimitRealizedTrade: BN,
    public realizedPnlForPositionNative: I80F48,
  ) {}

  isActive(): boolean {
    return this.marketIndex !== PerpPosition.PerpMarketIndexUnset;
  }

  public getBasePositionNative(perpMarket: PerpMarket): I80F48 {
    return I80F48.fromI64(this.basePositionLots.mul(perpMarket.baseLotSize));
  }

  public getBasePositionUi(
    perpMarket: PerpMarket,
    useEventQueue?: boolean,
  ): number {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    return perpMarket.baseLotsToUi(
      useEventQueue
        ? this.basePositionLots.add(this.takerBaseLots)
        : this.basePositionLots,
    );
  }

  public getQuotePositionUi(
    perpMarket: PerpMarket,
    useEventQueue?: boolean,
  ): number {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    const quotePositionUi = toUiDecimalsForQuote(this.quotePositionNative);

    return useEventQueue
      ? quotePositionUi + perpMarket.quoteLotsToUi(this.takerQuoteLots)
      : quotePositionUi;
  }

  public getNotionalValueUi(
    perpMarket: PerpMarket,
    useEventQueue?: boolean,
  ): number {
    return (
      this.getBasePositionUi(perpMarket, useEventQueue) * perpMarket.uiPrice
    );
  }

  public getUnsettledFunding(perpMarket: PerpMarket): I80F48 {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    if (this.basePositionLots.gt(new BN(0))) {
      return perpMarket.longFunding
        .sub(this.longSettledFunding)
        .mul(I80F48.fromI64(this.basePositionLots));
    } else if (this.basePositionLots.lt(new BN(0))) {
      return perpMarket.shortFunding
        .sub(this.shortSettledFunding)
        .mul(I80F48.fromI64(this.basePositionLots));
    }
    return ZERO_I80F48();
  }

  public getEquityUi(perpMarket: PerpMarket): number {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    return toUiDecimalsForQuote(this.getEquity(perpMarket));
  }

  public getEquity(perpMarket: PerpMarket): I80F48 {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    const lotsToQuote = I80F48.fromI64(perpMarket.baseLotSize).mul(
      perpMarket.price,
    );

    const baseLots = I80F48.fromI64(
      this.basePositionLots.add(this.takerBaseLots),
    );

    const unsettledFunding = this.getUnsettledFunding(perpMarket);
    const takerQuote = I80F48.fromI64(
      new BN(this.takerQuoteLots).mul(perpMarket.quoteLotSize),
    );
    const quoteCurrent = this.quotePositionNative
      .sub(unsettledFunding)
      .add(takerQuote);

    return baseLots.mul(lotsToQuote).add(quoteCurrent);
  }

  public hasOpenOrders(): boolean {
    const zero = new BN(0);
    return (
      !this.asksBaseLots.eq(zero) ||
      !this.bidsBaseLots.eq(zero) ||
      !this.takerBaseLots.eq(zero) ||
      !this.takerQuoteLots.eq(zero)
    );
  }

  public getAverageEntryPrice(perpMarket: PerpMarket): I80F48 {
    return I80F48.fromNumber(this.avgEntryPricePerBaseLot).div(
      I80F48.fromI64(perpMarket.baseLotSize),
    );
  }

  public getAverageEntryPriceUi(perpMarket: PerpMarket): number {
    return perpMarket.priceNativeToUi(
      this.getAverageEntryPrice(perpMarket).toNumber(),
    );
  }

  public getBreakEvenPriceUi(perpMarket: PerpMarket): number {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    if (this.basePositionLots.eq(new BN(0))) {
      return 0;
    }
    return perpMarket.priceNativeToUi(
      -this.quoteRunningNative.toNumber() /
        this.basePositionLots.mul(perpMarket.baseLotSize).toNumber(),
    );
  }

  public cumulativePnlOverPositionLifetimeUi(perpMarket: PerpMarket): number {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    const priceChange = perpMarket.price.sub(
      this.getAverageEntryPrice(perpMarket),
    );

    return toUiDecimalsForQuote(
      this.realizedPnlForPositionNative.add(
        this.getBasePositionNative(perpMarket).mul(priceChange),
      ),
    );
  }

  public getUnsettledPnl(perpMarket: PerpMarket): I80F48 {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    return this.quotePositionNative.add(
      this.getBasePositionNative(perpMarket).mul(perpMarket.price),
    );
  }

  public getUnsettledPnlUi(perpMarket: PerpMarket): number {
    return toUiDecimalsForQuote(this.getUnsettledPnl(perpMarket));
  }

  public updateSettleLimit(perpMarket: PerpMarket): void {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    const windowSize = perpMarket.settlePnlLimitWindowSizeTs;
    const windowStart = new BN(this.settlePnlLimitWindow).mul(windowSize);
    const windowEnd = windowStart.add(windowSize);
    const nowTs = new BN(Date.now() / 1000);
    const newWindow = nowTs.gte(windowEnd) || nowTs.lt(windowStart);
    if (newWindow) {
      this.settlePnlLimitWindow = nowTs.div(windowSize).toNumber();
      this.settlePnlLimitSettledInCurrentWindowNative = new BN(0);
    }
  }

  public availableSettleLimit(perpMarket: PerpMarket): [BN, BN] {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    if (perpMarket.settlePnlLimitFactor < 0) {
      return [RUST_I64_MIN(), RUST_I64_MAX()];
    }

    const baseNative = I80F48.fromI64(
      this.basePositionLots.mul(perpMarket.baseLotSize),
    );
    const positionValue = I80F48.fromNumber(
      perpMarket.stablePriceModel.stablePrice,
    )
      .mul(baseNative)
      .toNumber();
    const unrealized = new BN(perpMarket.settlePnlLimitFactor * positionValue);
    const used = new BN(
      this.settlePnlLimitSettledInCurrentWindowNative.toNumber(),
    );

    let minPnl = unrealized.neg().sub(used);
    let maxPnl = unrealized.sub(used);

    const realizedTrade = this.settlePnlLimitRealizedTrade;
    if (realizedTrade.gte(new BN(0))) {
      maxPnl = maxPnl.add(realizedTrade);
    } else {
      minPnl = minPnl.add(realizedTrade);
    }

    const realizedOther = new BN(this.realizedOtherPnlNative.toNumber());
    if (realizedOther.gte(new BN(0))) {
      maxPnl = maxPnl.add(realizedOther);
    } else {
      minPnl = minPnl.add(realizedOther);
    }

    return [BN.min(minPnl, new BN(0)), BN.max(maxPnl, new BN(0))];
  }

  public applyPnlSettleLimit(pnl: I80F48, perpMarket: PerpMarket): I80F48 {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }

    if (perpMarket.settlePnlLimitFactor < 0) {
      return pnl;
    }

    const [minPnl, maxPnl] = this.availableSettleLimit(perpMarket);
    if (pnl.lt(ZERO_I80F48())) {
      return pnl.max(I80F48.fromI64(minPnl));
    } else {
      return pnl.min(I80F48.fromI64(maxPnl));
    }
  }

  public getSettleablePnl(
    group: Group,
    perpMarket: PerpMarket,
    account: MangoAccount,
  ): I80F48 {
    if (perpMarket.perpMarketIndex !== this.marketIndex) {
      throw new Error("PerpPosition doesn't belong to the given market!");
    }
    this.updateSettleLimit(perpMarket);
    const perpSettleHealth = account.getPerpSettleHealth(group);
    const limitedUnsettled = this.applyPnlSettleLimit(
      this.getUnsettledPnl(perpMarket),
      perpMarket,
    );
    if (limitedUnsettled.lt(ZERO_I80F48())) {
      return limitedUnsettled.max(perpSettleHealth.max(ZERO_I80F48()).neg());
    }
    return limitedUnsettled;
  }

  getSettleablePnlUi(
    group: Group,
    perpMarket: PerpMarket,
    account: MangoAccount,
  ): number {
    return toUiDecimalsForQuote(
      this.getSettleablePnl(group, perpMarket, account),
    );
  }

  public canSettlePnl(
    group: Group,
    perpMarket: PerpMarket,
    account: MangoAccount,
  ): boolean {
    return !this.getSettleablePnl(group, perpMarket, account).eq(ZERO_I80F48());
  }

  toString(perpMarket?: PerpMarket): string {
    return perpMarket
      ? 'market - ' +
          perpMarket.name +
          ', basePositionLots - ' +
          perpMarket.baseLotsToUi(this.basePositionLots) +
          ', quotePositive - ' +
          toUiDecimalsForQuote(this.quotePositionNative.toNumber()) +
          ', bidsBaseLots - ' +
          perpMarket.baseLotsToUi(this.bidsBaseLots) +
          ', asksBaseLots - ' +
          perpMarket.baseLotsToUi(this.asksBaseLots) +
          ', takerBaseLots - ' +
          perpMarket.baseLotsToUi(this.takerBaseLots) +
          ', takerQuoteLots - ' +
          perpMarket.quoteLotsToUi(this.takerQuoteLots) +
          ', unsettled pnl - ' +
          this.getUnsettledPnlUi(perpMarket!).toString() +
          ', average entry price ui - ' +
          this.getAverageEntryPriceUi(perpMarket!).toString() +
          ', notional value ui - ' +
          this.getNotionalValueUi(perpMarket!).toString() +
          ', cumulative pnl over position lifetime ui - ' +
          this.cumulativePnlOverPositionLifetimeUi(perpMarket!).toString() +
          ', realized other pnl native ui - ' +
          toUiDecimalsForQuote(this.realizedOtherPnlNative) +
          ', cumulative long funding ui - ' +
          toUiDecimalsForQuote(this.cumulativeLongFunding) +
          ', cumulative short funding ui - ' +
          toUiDecimalsForQuote(this.cumulativeShortFunding)
      : '';
  }
}

export class PerpPositionDto {
  constructor(
    public marketIndex: number,
    public settlePnlLimitWindow: number,
    public settlePnlLimitSettledInCurrentWindowNative: BN,
    public basePositionLots: BN,
    public quotePositionNative: { val: BN },
    public quoteRunningNative: BN,
    public longSettledFunding: I80F48Dto,
    public shortSettledFunding: I80F48Dto,
    public bidsBaseLots: BN,
    public asksBaseLots: BN,
    public takerBaseLots: BN,
    public takerQuoteLots: BN,
    public cumulativeLongFunding: number,
    public cumulativeShortFunding: number,
    public makerVolume: BN,
    public takerVolume: BN,
    public perpSpotTransfers: BN,
    public avgEntryPricePerBaseLot: number,
    public realizedTradePnlNative: I80F48Dto,
    public realizedOtherPnlNative: I80F48Dto,
    public settlePnlLimitRealizedTrade: BN,
    public realizedPnlForPositionNative: I80F48Dto,
  ) {}
}

export class PerpOo {
  static OrderMarketUnset = 65535;
  static from(dto: PerpOoDto): PerpOo {
    return new PerpOo(dto.sideAndTree, dto.market, dto.clientId, dto.id);
  }

  constructor(
    public sideAndTree: any,
    public orderMarket: number,
    public clientId: BN,
    public id: BN,
  ) {}

  isActive(): boolean {
    return this.orderMarket !== PerpOo.OrderMarketUnset;
  }
}
export class PerpOoDto {
  constructor(
    public sideAndTree: any,
    public market: number,
    public clientId: BN,
    public id: BN,
  ) {}
}

export class HealthType {
  static maint = { maint: {} };
  static init = { init: {} };
  static liquidationEnd = { liquidationEnd: {} };
}
