import { BN } from '@project-serum/anchor';
import { OpenOrders } from '@project-serum/serum';
import { PublicKey } from '@solana/web3.js';
import _ from 'lodash';
import {
  HUNDRED_I80F48,
  I80F48,
  I80F48Dto,
  MAX_I80F48,
  ONE_I80F48,
  ZERO_I80F48,
} from '../numbers/I80F48';
import { Bank, BankForHealth, TokenIndex } from './bank';
import { Group } from './group';

import { HealthType, MangoAccount, PerpPosition } from './mangoAccount';
import { PerpMarket, PerpOrderSide } from './perp';
import { MarketIndex, Serum3Market, Serum3Side } from './serum3';

//               ░░░░
//
//                                           ██
//                                         ██░░██
// ░░          ░░                        ██░░░░░░██                            ░░░░
//                                     ██░░░░░░░░░░██
//                                     ██░░░░░░░░░░██
//                                   ██░░░░░░░░░░░░░░██
//                                 ██░░░░░░██████░░░░░░██
//                                 ██░░░░░░██████░░░░░░██
//                               ██░░░░░░░░██████░░░░░░░░██
//                               ██░░░░░░░░██████░░░░░░░░██
//                             ██░░░░░░░░░░██████░░░░░░░░░░██
//                           ██░░░░░░░░░░░░██████░░░░░░░░░░░░██
//                           ██░░░░░░░░░░░░██████░░░░░░░░░░░░██
//                         ██░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░██
//                         ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██
//                       ██░░░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░░░██
//                       ██░░░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░░░██
//                     ██░░░░░░░░░░░░░░░░░░██████░░░░░░░░░░░░░░░░░░██
//       ░░            ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██
//                       ██████████████████████████████████████████
// warning: this code is copy pasta from rust, keep in sync with health.rs

export class HealthCache {
  constructor(
    public tokenInfos: TokenInfo[],
    public serum3Infos: Serum3Info[],
    public perpInfos: PerpInfo[],
  ) {}

  static fromMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
  ): HealthCache {
    // token contribution from token accounts
    const tokenInfos = mangoAccount.tokensActive().map((tokenPosition) => {
      const bank = group.getFirstBankByTokenIndex(tokenPosition.tokenIndex);
      return TokenInfo.fromBank(bank, tokenPosition.balance(bank));
    });

    // Fill the TokenInfo balance with free funds in serum3 oo accounts, and fill
    // the serum3MaxReserved with their reserved funds. Also build Serum3Infos.
    const serum3Infos = mangoAccount.serum3Active().map((serum3) => {
      const oo = mangoAccount.getSerum3OoAccount(serum3.marketIndex);

      // find the TokenInfos for the market's base and quote tokens
      const baseIndex = tokenInfos.findIndex(
        (tokenInfo) => tokenInfo.tokenIndex === serum3.baseTokenIndex,
      );
      const baseInfo = tokenInfos[baseIndex];
      if (!baseInfo) {
        throw new Error(
          `BaseInfo not found for market with marketIndex ${serum3.marketIndex}!`,
        );
      }
      const quoteIndex = tokenInfos.findIndex(
        (tokenInfo) => tokenInfo.tokenIndex === serum3.quoteTokenIndex,
      );
      const quoteInfo = tokenInfos[quoteIndex];
      if (!quoteInfo) {
        throw new Error(
          `QuoteInfo not found for market with marketIndex ${serum3.marketIndex}!`,
        );
      }

      return Serum3Info.fromOoModifyingTokenInfos(
        baseIndex,
        baseInfo,
        quoteIndex,
        quoteInfo,
        serum3.marketIndex,
        oo,
      );
    });

    // health contribution from perp accounts
    const perpInfos = mangoAccount.perpActive().map((perpPosition) => {
      const perpMarket = group.getPerpMarketByMarketIndex(
        perpPosition.marketIndex,
      );
      return PerpInfo.fromPerpPosition(perpMarket, perpPosition);
    });

    return new HealthCache(tokenInfos, serum3Infos, perpInfos);
  }

  static fromDto(dto): HealthCache {
    return new HealthCache(
      dto.tokenInfos.map((dto) => TokenInfo.fromDto(dto)),
      dto.serum3Infos.map((dto) => Serum3Info.fromDto(dto)),
      dto.perpInfos.map((dto) => PerpInfo.fromDto(dto)),
    );
  }

  computeSerum3Reservations(healthType: HealthType): {
    tokenMaxReserved: I80F48[];
    serum3Reserved: Serum3Reserved[];
  } {
    // For each token, compute the sum of serum-reserved amounts over all markets.
    const tokenMaxReserved = new Array(this.tokenInfos.length)
      .fill(null)
      .map((ignored) => ZERO_I80F48());

    // For each serum market, compute what happened if reserved_base was converted to quote
    // or reserved_quote was converted to base.
    const serum3Reserved: Serum3Reserved[] = [];

    for (const info of this.serum3Infos) {
      const quote = this.tokenInfos[info.quoteIndex];
      const base = this.tokenInfos[info.baseIndex];

      const reservedBase = info.reservedBase;
      const reservedQuote = info.reservedQuote;

      const quoteAsset = quote.prices.asset(healthType);
      const baseLiab = base.prices.liab(healthType);
      const allReservedAsBase = reservedBase.add(
        reservedQuote.mul(quoteAsset).div(baseLiab),
      );
      const baseAsset = base.prices.asset(healthType);
      const quoteLiab = quote.prices.liab(healthType);
      const allReservedAsQuote = reservedQuote.add(
        reservedBase.mul(baseAsset).div(quoteLiab),
      );

      const baseMaxReserved = tokenMaxReserved[info.baseIndex];
      baseMaxReserved.iadd(allReservedAsBase);
      const quoteMaxReserved = tokenMaxReserved[info.quoteIndex];
      quoteMaxReserved.iadd(allReservedAsQuote);

      serum3Reserved.push(
        new Serum3Reserved(allReservedAsBase, allReservedAsQuote),
      );
    }

    return {
      tokenMaxReserved: tokenMaxReserved,
      serum3Reserved: serum3Reserved,
    };
  }

  public health(healthType: HealthType): I80F48 {
    const health = ZERO_I80F48();
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      // console.log(` - ti ${contrib}`);
      health.iadd(contrib);
    }
    const res = this.computeSerum3Reservations(healthType);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      // console.log(` - si ${contrib}`);
      health.iadd(contrib);
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      // console.log(` - pi ${contrib}`);
      health.iadd(contrib);
    }
    return health;
  }

  // Note: only considers positive perp pnl contributions, see program code for more reasoning
  public perpSettleHealth(): I80F48 {
    const health = ZERO_I80F48();
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(HealthType.maint);
      // console.log(` - ti ${contrib}`);
      health.iadd(contrib);
    }
    const res = this.computeSerum3Reservations(HealthType.maint);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        HealthType.maint,
        this.tokenInfos,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      // console.log(` - si ${contrib}`);
      health.iadd(contrib);
    }
    for (const perpInfo of this.perpInfos) {
      if (perpInfo.trustedMarket) {
        const positiveContrib = perpInfo
          .uncappedHealthContribution(HealthType.maint)
          .max(ZERO_I80F48());
        // console.log(` - pi ${positiveContrib}`);
        health.iadd(positiveContrib);
      }
    }
    return health;
  }

  public assets(healthType: HealthType): I80F48 {
    const assets = ZERO_I80F48();
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets.iadd(contrib);
      }
    }
    const res = this.computeSerum3Reservations(HealthType.maint);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      if (contrib.isPos()) {
        assets.iadd(contrib);
      }
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets.iadd(contrib);
      }
    }
    return assets;
  }

  public liabs(healthType: HealthType): I80F48 {
    const liabs = ZERO_I80F48();
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      if (contrib.isNeg()) {
        liabs.isub(contrib);
      }
    }
    const res = this.computeSerum3Reservations(HealthType.maint);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      if (contrib.isNeg()) {
        liabs.isub(contrib);
      }
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      if (contrib.isNeg()) {
        liabs.isub(contrib);
      }
    }
    return liabs;
  }

  public healthRatio(healthType: HealthType): I80F48 {
    const assets = ZERO_I80F48();
    const liabs = ZERO_I80F48();

    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets.iadd(contrib);
      } else {
        liabs.isub(contrib);
      }
    }
    const res = this.computeSerum3Reservations(HealthType.maint);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      if (contrib.isPos()) {
        assets.iadd(contrib);
      } else {
        liabs.isub(contrib);
      }
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets.iadd(contrib);
      } else {
        liabs.isub(contrib);
      }
    }

    // console.log(` - assets ${assets}, liabs ${liabs}`);

    if (liabs.gt(I80F48.fromNumber(0.001))) {
      return HUNDRED_I80F48().mul(assets.sub(liabs).div(liabs));
    } else {
      return MAX_I80F48();
    }
  }

  findTokenInfoIndex(tokenIndex: TokenIndex): number {
    return this.tokenInfos.findIndex(
      (tokenInfo) => tokenInfo.tokenIndex === tokenIndex,
    );
  }

  getOrCreateTokenInfoIndex(bank: BankForHealth): number {
    const index = this.findTokenInfoIndex(bank.tokenIndex);
    if (index == -1) {
      this.tokenInfos.push(TokenInfo.fromBank(bank));
    }
    return this.findTokenInfoIndex(bank.tokenIndex);
  }

  simHealthRatioWithTokenPositionChanges(
    group: Group,
    nativeTokenChanges: {
      nativeTokenAmount: I80F48;
      mintPk: PublicKey;
    }[],
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    const adjustedCache: HealthCache = _.cloneDeep(this);
    // HealthCache.logHealthCache('beforeChange', adjustedCache);
    for (const change of nativeTokenChanges) {
      const bank: Bank = group.getFirstBankByMint(change.mintPk);
      const changeIndex = adjustedCache.getOrCreateTokenInfoIndex(bank);
      // TODO: this will no longer work as easily because of the health weight changes
      adjustedCache.tokenInfos[changeIndex].balanceNative.iadd(
        change.nativeTokenAmount,
      );
    }
    // HealthCache.logHealthCache('afterChange', adjustedCache);
    return adjustedCache.healthRatio(healthType);
  }

  findSerum3InfoIndex(marketIndex: MarketIndex): number {
    return this.serum3Infos.findIndex(
      (serum3Info) => serum3Info.marketIndex === marketIndex,
    );
  }

  getOrCreateSerum3InfoIndex(
    baseBank: BankForHealth,
    quoteBank: BankForHealth,
    serum3Market: Serum3Market,
  ): number {
    const index = this.findSerum3InfoIndex(serum3Market.marketIndex);
    const baseEntryIndex = this.getOrCreateTokenInfoIndex(baseBank);
    const quoteEntryIndex = this.getOrCreateTokenInfoIndex(quoteBank);
    if (index == -1) {
      this.serum3Infos.push(
        Serum3Info.emptyFromSerum3Market(
          serum3Market,
          baseEntryIndex,
          quoteEntryIndex,
        ),
      );
    }
    return this.findSerum3InfoIndex(serum3Market.marketIndex);
  }

  adjustSerum3Reserved(
    baseBank: BankForHealth,
    quoteBank: BankForHealth,
    serum3Market: Serum3Market,
    reservedBaseChange: I80F48,
    freeBaseChange: I80F48,
    reservedQuoteChange: I80F48,
    freeQuoteChange: I80F48,
  ): void {
    const baseEntryIndex = this.getOrCreateTokenInfoIndex(baseBank);
    const quoteEntryIndex = this.getOrCreateTokenInfoIndex(quoteBank);

    const baseEntry = this.tokenInfos[baseEntryIndex];
    const quoteEntry = this.tokenInfos[quoteEntryIndex];

    // Apply it to the tokens
    baseEntry.balanceNative.iadd(freeBaseChange);
    quoteEntry.balanceNative.iadd(freeQuoteChange);

    // Apply it to the serum3 info
    const index = this.getOrCreateSerum3InfoIndex(
      baseBank,
      quoteBank,
      serum3Market,
    );
    const serum3Info = this.serum3Infos[index];
    serum3Info.reservedBase.iadd(reservedBaseChange);
    serum3Info.reservedQuote.iadd(reservedQuoteChange);
  }

  simHealthRatioWithSerum3BidChanges(
    baseBank: BankForHealth,
    quoteBank: BankForHealth,
    bidNativeQuoteAmount: I80F48,
    serum3Market: Serum3Market,
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    const adjustedCache: HealthCache = _.cloneDeep(this);
    const quoteIndex = adjustedCache.getOrCreateTokenInfoIndex(quoteBank);

    // Move token balance to reserved funds in open orders,
    // essentially simulating a place order

    // Reduce token balance for quote
    adjustedCache.tokenInfos[quoteIndex].balanceNative.isub(
      bidNativeQuoteAmount,
    );

    // Increase reserved in Serum3Info for quote
    adjustedCache.adjustSerum3Reserved(
      baseBank,
      quoteBank,
      serum3Market,
      ZERO_I80F48(),
      ZERO_I80F48(),
      bidNativeQuoteAmount,
      ZERO_I80F48(),
    );
    return adjustedCache.healthRatio(healthType);
  }

  simHealthRatioWithSerum3AskChanges(
    baseBank: BankForHealth,
    quoteBank: BankForHealth,
    askNativeBaseAmount: I80F48,
    serum3Market: Serum3Market,
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    const adjustedCache: HealthCache = _.cloneDeep(this);
    const baseIndex = adjustedCache.getOrCreateTokenInfoIndex(baseBank);

    // Move token balance to reserved funds in open orders,
    // essentially simulating a place order

    // Reduce token balance for base
    adjustedCache.tokenInfos[baseIndex].balanceNative.isub(askNativeBaseAmount);

    // Increase reserved in Serum3Info for base
    adjustedCache.adjustSerum3Reserved(
      baseBank,
      quoteBank,
      serum3Market,
      askNativeBaseAmount,
      ZERO_I80F48(),
      ZERO_I80F48(),
      ZERO_I80F48(),
    );
    return adjustedCache.healthRatio(healthType);
  }

  findPerpInfoIndex(perpMarketIndex: number): number {
    return this.perpInfos.findIndex(
      (perpInfo) => perpInfo.perpMarketIndex === perpMarketIndex,
    );
  }

  getOrCreatePerpInfoIndex(perpMarket: PerpMarket): number {
    const index = this.findPerpInfoIndex(perpMarket.perpMarketIndex);
    if (index == -1) {
      this.perpInfos.push(PerpInfo.emptyFromPerpMarket(perpMarket));
    }
    return this.findPerpInfoIndex(perpMarket.perpMarketIndex);
  }

  adjustPerpInfo(
    perpInfoIndex: number,
    price: I80F48,
    side: PerpOrderSide,
    newOrderBaseLots: BN,
  ): void {
    if (side == PerpOrderSide.bid) {
      this.perpInfos[perpInfoIndex].baseLots.iadd(newOrderBaseLots);
      this.perpInfos[perpInfoIndex].quote.isub(
        I80F48.fromI64(newOrderBaseLots)
          .mul(I80F48.fromI64(this.perpInfos[perpInfoIndex].baseLotSize))
          .mul(price),
      );
    } else {
      this.perpInfos[perpInfoIndex].baseLots.isub(newOrderBaseLots);
      this.perpInfos[perpInfoIndex].quote.iadd(
        I80F48.fromI64(newOrderBaseLots)
          .mul(I80F48.fromI64(this.perpInfos[perpInfoIndex].baseLotSize))
          .mul(price),
      );
    }
  }

  simHealthRatioWithPerpOrderChanges(
    perpMarket: PerpMarket,
    existingPerpPosition: PerpPosition,
    side: PerpOrderSide,
    baseLots: BN,
    price: I80F48,
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    const clonedHealthCache: HealthCache = _.cloneDeep(this);
    const perpInfoIndex =
      clonedHealthCache.getOrCreatePerpInfoIndex(perpMarket);
    clonedHealthCache.adjustPerpInfo(perpInfoIndex, price, side, baseLots);
    return clonedHealthCache.healthRatio(healthType);
  }

  public logHealthCache(debug: string): void {
    if (debug) console.log(debug);
    for (const token of this.tokenInfos) {
      console.log(` ${token.toString()}`);
    }
    const res = this.computeSerum3Reservations(HealthType.maint);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      console.log(
        ` ${serum3Info.toString(
          this.tokenInfos,
          res.tokenMaxReserved,
          res.serum3Reserved[index],
        )}`,
      );
    }
    console.log(
      ` assets ${this.assets(HealthType.init)}, liabs ${this.liabs(
        HealthType.init,
      )}, `,
    );
    console.log(` health(HealthType.init) ${this.health(HealthType.init)}`);
    console.log(
      ` healthRatio(HealthType.init) ${this.healthRatio(HealthType.init)}`,
    );
  }

  private static scanRightUntilLessThan(
    start: I80F48,
    target: I80F48,
    fun: (amount: I80F48) => I80F48,
  ): I80F48 {
    const maxIterations = 20;
    let current = start;
    for (const key of Array(maxIterations).fill(0).keys()) {
      const value = fun(current);
      if (value.lt(target)) {
        return current;
      }
      current = current.max(ONE_I80F48()).mul(I80F48.fromNumber(2));
    }
    throw new Error('Could not find amount that led to health ratio <=0');
  }

  private static binaryApproximationSearch(
    left: I80F48,
    leftValue: I80F48,
    right: I80F48,
    targetValue: I80F48,
    minStep: I80F48,
    fun: (I80F48) => I80F48,
  ): I80F48 {
    const maxIterations = 20;
    const targetError = I80F48.fromNumber(0.1);
    const rightValue = fun(right);

    if (
      (leftValue.sub(targetValue).isPos() &&
        rightValue.sub(targetValue).isPos()) ||
      (leftValue.sub(targetValue).isNeg() &&
        rightValue.sub(targetValue).isNeg())
    ) {
      throw new Error(
        `Internal error: left ${leftValue.toNumber()}  and right ${rightValue.toNumber()} don't contain the target value ${targetValue.toNumber()}!`,
      );
    }

    let newAmount;
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    for (const key of Array(maxIterations).fill(0).keys()) {
      if (right.sub(left).abs().lt(minStep)) {
        return left;
      }
      newAmount = left.add(right).mul(I80F48.fromNumber(0.5));
      const newAmountRatio = fun(newAmount);
      const error = newAmountRatio.sub(targetValue);
      if (error.isPos() && error.lt(targetError)) {
        return newAmount;
      }
      if (newAmountRatio.gt(targetValue) != rightValue.gt(targetValue)) {
        left = newAmount;
      } else {
        right = newAmount;
      }
    }

    console.error(
      `Unable to get targetRatio within ${maxIterations} iterations`,
    );
    return newAmount;
  }

  getMaxSourceForTokenSwap(
    sourceBank: BankForHealth,
    targetBank: BankForHealth,
    price: I80F48,
    minRatio: I80F48,
  ): I80F48 {
    if (
      sourceBank.initLiabWeight
        .sub(targetBank.initAssetWeight)
        .abs()
        .lte(ZERO_I80F48())
    ) {
      return ZERO_I80F48();
    }

    // The health_ratio is a nonlinear based on swap amount.
    // For large swap amounts the slope is guaranteed to be negative, but small amounts
    // can have positive slope (e.g. using source deposits to pay back target borrows).
    //
    // That means:
    // - even if the initial ratio is < minRatio it can be useful to swap to *increase* health
    // - be careful about finding the minRatio point: the function isn't convex

    const initialRatio = this.healthRatio(HealthType.init);
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const initialHealth = this.health(HealthType.init);
    if (initialRatio.lte(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    const healthCacheClone: HealthCache = _.cloneDeep(this);
    const sourceIndex = healthCacheClone.getOrCreateTokenInfoIndex(sourceBank);
    const targetIndex = healthCacheClone.getOrCreateTokenInfoIndex(targetBank);
    const source = healthCacheClone.tokenInfos[sourceIndex];
    const target = healthCacheClone.tokenInfos[targetIndex];

    // If the price is sufficiently good, then health will just increase from swapping:
    // once we've swapped enough, swapping x reduces health by x * source_liab_weight and
    // increases it by x * target_asset_weight * price_factor.
    const finalHealthSlope = source.initLiabWeight
      .neg()
      .mul(source.prices.liab(HealthType.init))
      .add(
        target.initAssetWeight
          .mul(target.prices.asset(HealthType.init))
          .mul(price),
      );

    if (finalHealthSlope.gte(ZERO_I80F48())) {
      return MAX_I80F48();
    }

    // There are two key slope changes: Assume source.balance > 0 and target.balance < 0. Then
    // initially health ratio goes up. When one of balances flips sign, the health ratio slope
    // may be positive or negative for a bit, until both balances have flipped and the slope is
    // negative.
    // The maximum will be at one of these points (ignoring serum3 effects).

    function cacheAfterSwap(amount: I80F48): HealthCache {
      const adjustedCache: HealthCache = _.cloneDeep(healthCacheClone);
      // adjustedCache.logHealthCache('beforeSwap', adjustedCache);
      // TODO: make a copy of the bank, apply amount, recompute weights,
      // and set the new weights on the tokenInfos
      adjustedCache.tokenInfos[sourceIndex].balanceNative.isub(amount);
      adjustedCache.tokenInfos[targetIndex].balanceNative.iadd(
        amount.mul(price),
      );
      // adjustedCache.logHealthCache('afterSwap', adjustedCache);
      return adjustedCache;
    }

    function healthRatioAfterSwap(amount: I80F48): I80F48 {
      return cacheAfterSwap(amount).healthRatio(HealthType.init);
    }

    function healthAfterSwap(amount: I80F48): I80F48 {
      return cacheAfterSwap(amount).health(HealthType.init);
    }

    // There are two key slope changes: Assume source.balance > 0 and target.balance < 0.
    // When these values flip sign, the health slope decreases, but could still be positive.
    // After point1 it's definitely negative (due to finalHealthSlope check above).
    // The maximum health ratio will be at 0 or at one of these points (ignoring serum3 effects).
    const sourceForZeroTargetBalance = target.balanceNative.neg().div(price);
    const point0Amount = source.balanceNative
      .min(sourceForZeroTargetBalance)
      .max(ZERO_I80F48());
    const point1Amount = source.balanceNative
      .max(sourceForZeroTargetBalance)
      .max(ZERO_I80F48());
    const cache0 = cacheAfterSwap(point0Amount);
    const point0Ratio = cache0.healthRatio(HealthType.init);
    const point0Health = cache0.health(HealthType.init);
    const cache1 = cacheAfterSwap(point1Amount);
    const point1Ratio = cache1.healthRatio(HealthType.init);
    const point1Health = cache1.health(HealthType.init);

    let amount: I80F48;

    if (
      initialRatio.lte(minRatio) &&
      point0Ratio.lt(minRatio) &&
      point1Ratio.lt(minRatio)
    ) {
      // If we have to stay below the target ratio, pick the highest one
      if (point0Ratio.gt(initialRatio)) {
        if (point1Ratio.gt(point0Ratio)) {
          amount = point1Amount;
        } else {
          amount = point0Amount;
        }
      } else if (point1Ratio.gt(initialRatio)) {
        amount = point1Amount;
      } else {
        amount = ZERO_I80F48();
      }
    } else if (point1Ratio.gte(minRatio)) {
      // If point1Ratio is still bigger than minRatio, the target amount must be >point1Amount
      // search to the right of point1Amount: but how far?
      // At point1, source.balance < 0 and target.balance > 0, so use a simple estimation for
      // zero health: health - source_liab_weight * a + target_asset_weight * a * priceFactor = 0.
      // where a is the source token native amount.
      // Note that this is just an estimate. Swapping can increase the amount that serum3
      // reserved contributions offset, moving the actual zero point further to the right.
      if (point1Health.lte(ZERO_I80F48())) {
        return ZERO_I80F48();
      }
      const zeroHealthEstimate = point1Amount.sub(
        point1Health.sub(finalHealthSlope),
      );
      const rightBound = HealthCache.scanRightUntilLessThan(
        zeroHealthEstimate,
        minRatio,
        healthRatioAfterSwap,
      );
      if (rightBound.eq(zeroHealthEstimate)) {
        amount = HealthCache.binaryApproximationSearch(
          point1Amount,
          point1Ratio,
          rightBound,
          minRatio,
          ZERO_I80F48(),
          healthRatioAfterSwap,
        );
      } else {
        amount = HealthCache.binaryApproximationSearch(
          zeroHealthEstimate,
          healthRatioAfterSwap(zeroHealthEstimate),
          rightBound,
          minRatio,
          ZERO_I80F48(),
          healthRatioAfterSwap,
        );
      }
    } else if (point0Ratio.gte(minRatio)) {
      // Must be between point0Amount and point1Amount.
      amount = HealthCache.binaryApproximationSearch(
        point0Amount,
        point0Ratio,
        point1Amount,
        minRatio,
        ZERO_I80F48(),
        healthRatioAfterSwap,
      );
    } else {
      // Must be between 0 and point0_amount
      amount = HealthCache.binaryApproximationSearch(
        ZERO_I80F48(),
        initialRatio,
        point0Amount,
        minRatio,
        ZERO_I80F48(),
        healthRatioAfterSwap,
      );
    }

    return amount;
  }

  getMaxSerum3OrderForHealthRatio(
    baseBank: BankForHealth,
    quoteBank: BankForHealth,
    serum3Market: Serum3Market,
    side: Serum3Side,
    minRatio: I80F48,
  ): I80F48 {
    const healthCacheClone: HealthCache = _.cloneDeep(this);

    const baseIndex = healthCacheClone.getOrCreateTokenInfoIndex(baseBank);
    const quoteIndex = healthCacheClone.getOrCreateTokenInfoIndex(quoteBank);
    const base = healthCacheClone.tokenInfos[baseIndex];
    const quote = healthCacheClone.tokenInfos[quoteIndex];

    // Binary search between current health (0 sized new order) and
    // an amount to trade which will bring health to 0.

    // Current health and amount i.e. 0
    const initialAmount = ZERO_I80F48();
    const initialHealth = this.health(HealthType.init);
    const initialRatio = this.healthRatio(HealthType.init);
    if (initialRatio.lte(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    // Amount which would bring health to 0
    // where M = max(A_deposits, B_borrows)
    // amount = M + (init_health + M * (B_init_liab - A_init_asset)) / (A_init_liab - B_init_asset);
    // A is what we would be essentially swapping for B
    // So when its an ask, then base->quote,
    // and when its a bid, then quote->bid
    let zeroAmount;
    if (side == Serum3Side.ask) {
      const quoteBorrows = quote.balanceNative.lt(ZERO_I80F48())
        ? quote.balanceNative.abs().mul(quote.prices.liab(HealthType.init))
        : ZERO_I80F48();
      const max = base.balanceNative
        .mul(base.prices.asset(HealthType.init))
        .max(quoteBorrows);
      zeroAmount = max.add(
        initialHealth
          .add(max.mul(quote.initLiabWeight.sub(base.initAssetWeight)))
          .div(
            base
              .liabWeight(HealthType.init)
              .sub(quote.assetWeight(HealthType.init)),
          ),
      );
    } else {
      const baseBorrows = base.balanceNative.lt(ZERO_I80F48())
        ? base.balanceNative.abs().mul(base.prices.liab(HealthType.init))
        : ZERO_I80F48();
      const max = quote.balanceNative
        .mul(quote.prices.asset(HealthType.init))
        .max(baseBorrows);
      zeroAmount = max.add(
        initialHealth
          .add(max.mul(base.initLiabWeight.sub(quote.initAssetWeight)))
          .div(
            quote
              .liabWeight(HealthType.init)
              .sub(base.assetWeight(HealthType.init)),
          ),
      );
    }

    const cache = cacheAfterPlacingOrder(zeroAmount);
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const zeroAmountHealth = cache.health(HealthType.init);
    const zeroAmountRatio = cache.healthRatio(HealthType.init);

    function cacheAfterPlacingOrder(amount: I80F48): HealthCache {
      const adjustedCache: HealthCache = _.cloneDeep(healthCacheClone);
      // adjustedCache.logHealthCache(` before placing order ${amount}`);
      // TODO: there should also be some issue with oracle vs stable price here;
      // probably better to pass in not the quote amount but the base or quote native amount
      side === Serum3Side.ask
        ? adjustedCache.tokenInfos[baseIndex].balanceNative.isub(
            amount.div(base.prices.oracle),
          )
        : adjustedCache.tokenInfos[quoteIndex].balanceNative.isub(
            amount.div(quote.prices.oracle),
          );
      adjustedCache.adjustSerum3Reserved(
        baseBank,
        quoteBank,
        serum3Market,
        side === Serum3Side.ask
          ? amount.div(base.prices.oracle)
          : ZERO_I80F48(),
        ZERO_I80F48(),
        side === Serum3Side.bid
          ? amount.div(quote.prices.oracle)
          : ZERO_I80F48(),
        ZERO_I80F48(),
      );
      // adjustedCache.logHealthCache(' after placing order');

      return adjustedCache;
    }

    function healthRatioAfterPlacingOrder(amount: I80F48): I80F48 {
      return cacheAfterPlacingOrder(amount).healthRatio(HealthType.init);
    }

    const amount = HealthCache.binaryApproximationSearch(
      initialAmount,
      initialRatio,
      zeroAmount,
      minRatio,
      ONE_I80F48(),
      healthRatioAfterPlacingOrder,
    );

    return amount;
  }

  getMaxPerpForHealthRatio(
    perpMarket: PerpMarket,
    price,
    side: PerpOrderSide,
    minRatio: I80F48,
  ): I80F48 {
    const healthCacheClone: HealthCache = _.cloneDeep(this);

    const initialRatio = this.healthRatio(HealthType.init);
    if (initialRatio.lt(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    const direction = side == PerpOrderSide.bid ? 1 : -1;

    const perpInfoIndex = healthCacheClone.getOrCreatePerpInfoIndex(perpMarket);
    const perpInfo = healthCacheClone.perpInfos[perpInfoIndex];
    const prices = perpInfo.prices;
    const baseLotSize = I80F48.fromI64(perpMarket.baseLotSize);

    // If the price is sufficiently good then health will just increase from trading
    const finalHealthSlope =
      direction == 1
        ? perpInfo.initAssetWeight.mul(prices.asset(HealthType.init)).sub(price)
        : price.sub(perpInfo.initLiabWeight.mul(prices.liab(HealthType.init)));
    if (finalHealthSlope.gte(ZERO_I80F48())) {
      return MAX_I80F48();
    }

    function cacheAfterTrade(baseLots: BN): HealthCache {
      const adjustedCache: HealthCache = _.cloneDeep(healthCacheClone);
      // adjustedCache.logHealthCache(' -- before trade');
      adjustedCache.adjustPerpInfo(perpInfoIndex, price, side, baseLots);
      // adjustedCache.logHealthCache(' -- after trade');
      return adjustedCache;
    }

    function healthAfterTrade(baseLots: I80F48): I80F48 {
      return cacheAfterTrade(new BN(baseLots.toNumber())).health(
        HealthType.init,
      );
    }
    function healthRatioAfterTrade(baseLots: I80F48): I80F48 {
      return cacheAfterTrade(new BN(baseLots.toNumber())).healthRatio(
        HealthType.init,
      );
    }
    function healthRatioAfterTradeTrunc(baseLots: I80F48): I80F48 {
      return healthRatioAfterTrade(baseLots.floor());
    }

    const initialBaseLots = I80F48.fromU64(perpInfo.baseLots);

    // There are two cases:
    // 1. We are increasing abs(baseLots)
    // 2. We are bringing the base position to 0, and then going to case 1.
    const hasCase2 =
      (initialBaseLots.gt(ZERO_I80F48()) && direction == -1) ||
      (initialBaseLots.lt(ZERO_I80F48()) && direction == 1);

    let case1Start: I80F48, case1StartRatio: I80F48;
    if (hasCase2) {
      case1Start = initialBaseLots.abs();
      case1StartRatio = healthRatioAfterTrade(case1Start);
    } else {
      case1Start = ZERO_I80F48();
      case1StartRatio = initialRatio;
    }

    // If we start out below minRatio and can't go above, pick the best case
    let baseLots: I80F48;
    if (initialRatio.lte(minRatio) && case1StartRatio.lt(minRatio)) {
      if (case1StartRatio.gte(initialRatio)) {
        baseLots = case1Start;
      } else {
        baseLots = ZERO_I80F48();
      }
    } else if (case1StartRatio.gte(minRatio)) {
      // Must reach minRatio to the right of case1Start

      // Need to figure out how many lots to trade to reach zero health (zero_health_amount).
      // We do this by looking at the starting health and the health slope per
      // traded base lot (final_health_slope).
      const startCache = cacheAfterTrade(new BN(case1Start.toNumber()));
      const startHealth = startCache.health(HealthType.init);
      if (startHealth.lte(ZERO_I80F48())) {
        return ZERO_I80F48();
      }

      // The perp market's contribution to the health above may be capped. But we need to trade
      // enough to fully reduce any positive-pnl buffer. Thus get the uncapped health:
      const perpInfo = startCache.perpInfos[perpInfoIndex];
      const startHealthUncapped = startHealth
        .sub(perpInfo.healthContribution(HealthType.init))
        .add(perpInfo.uncappedHealthContribution(HealthType.init));

      const zeroHealthAmount = case1Start
        .sub(startHealthUncapped.div(finalHealthSlope).div(baseLotSize))
        .add(ONE_I80F48());
      const zeroHealthRatio = healthRatioAfterTradeTrunc(zeroHealthAmount);

      baseLots = HealthCache.binaryApproximationSearch(
        case1Start,
        case1StartRatio,
        zeroHealthAmount,
        minRatio,
        ONE_I80F48(),
        healthRatioAfterTradeTrunc,
      );
    } else {
      // Between 0 and case1Start
      baseLots = HealthCache.binaryApproximationSearch(
        ZERO_I80F48(),
        initialRatio,
        case1Start,
        minRatio,
        ONE_I80F48(),
        healthRatioAfterTradeTrunc,
      );
    }

    return baseLots.floor();
  }
}

export class Prices {
  constructor(public oracle: I80F48, public stable: I80F48) {}

  public liab(healthType: HealthType): I80F48 {
    if (healthType == HealthType.maint) {
      return this.oracle;
    }
    return this.oracle.max(this.stable);
  }

  public asset(healthType: HealthType): I80F48 {
    if (healthType == HealthType.maint) {
      return this.oracle;
    }
    return this.oracle.min(this.stable);
  }
}

export class TokenInfo {
  constructor(
    public tokenIndex: TokenIndex,
    public maintAssetWeight: I80F48,
    public initAssetWeight: I80F48,
    public maintLiabWeight: I80F48,
    public initLiabWeight: I80F48,
    public prices: Prices,
    public balanceNative: I80F48,
  ) {}

  static fromDto(dto: TokenInfoDto): TokenInfo {
    return new TokenInfo(
      dto.tokenIndex as TokenIndex,
      I80F48.from(dto.maintAssetWeight),
      I80F48.from(dto.initAssetWeight),
      I80F48.from(dto.maintLiabWeight),
      I80F48.from(dto.initLiabWeight),
      new Prices(
        I80F48.from(dto.prices.oracle),
        I80F48.from(dto.prices.stable),
      ),
      I80F48.from(dto.balanceNative),
    );
  }

  static fromBank(bank: BankForHealth, nativeBalance?: I80F48): TokenInfo {
    return new TokenInfo(
      bank.tokenIndex,
      bank.maintAssetWeight,
      bank.scaledInitAssetWeight(),
      bank.maintLiabWeight,
      bank.scaledInitLiabWeight(),
      new Prices(
        bank.price,
        I80F48.fromNumber(bank.stablePriceModel.stablePrice),
      ),
      nativeBalance ? nativeBalance : ZERO_I80F48(),
    );
  }

  assetWeight(healthType: HealthType): I80F48 {
    return healthType == HealthType.init
      ? this.initAssetWeight
      : this.maintAssetWeight;
  }

  liabWeight(healthType: HealthType): I80F48 {
    return healthType == HealthType.init
      ? this.initLiabWeight
      : this.maintLiabWeight;
  }

  healthContribution(healthType: HealthType): I80F48 {
    let weight, price;
    if (this.balanceNative.isNeg()) {
      weight = this.liabWeight(healthType);
      price = this.prices.liab(healthType);
    } else {
      weight = this.assetWeight(healthType);
      price = this.prices.asset(healthType);
    }
    return this.balanceNative.mul(weight).mul(price);
  }

  toString(): string {
    return `  tokenIndex: ${this.tokenIndex}, balanceNative: ${
      this.balanceNative
    }, initHealth ${this.healthContribution(HealthType.init)}`;
  }
}

export class Serum3Reserved {
  constructor(
    public allReservedAsBase: I80F48,
    public allReservedAsQuote: I80F48,
  ) {}
}

export class Serum3Info {
  constructor(
    public reservedBase: I80F48,
    public reservedQuote: I80F48,
    public baseIndex: number,
    public quoteIndex: number,
    public marketIndex: MarketIndex,
  ) {}

  static fromDto(dto: Serum3InfoDto): Serum3Info {
    return new Serum3Info(
      I80F48.from(dto.reservedBase),
      I80F48.from(dto.reservedQuote),
      dto.baseIndex,
      dto.quoteIndex,
      dto.marketIndex as MarketIndex,
    );
  }

  static emptyFromSerum3Market(
    serum3Market: Serum3Market,
    baseEntryIndex: number,
    quoteEntryIndex: number,
  ): Serum3Info {
    return new Serum3Info(
      ZERO_I80F48(),
      ZERO_I80F48(),
      baseEntryIndex,
      quoteEntryIndex,
      serum3Market.marketIndex,
    );
  }

  static fromOoModifyingTokenInfos(
    baseIndex: number,
    baseInfo: TokenInfo,
    quoteIndex: number,
    quoteInfo: TokenInfo,
    marketIndex: MarketIndex,
    oo: OpenOrders,
  ): Serum3Info {
    // add the amounts that are freely settleable immediately to token balances
    const baseFree = I80F48.fromI64(oo.baseTokenFree);
    // NOTE: referrerRebatesAccrued is not declared on oo class, but the layout
    // is aware of it
    const quoteFree = I80F48.fromI64(
      oo.quoteTokenFree.add((oo as any).referrerRebatesAccrued),
    );
    baseInfo.balanceNative.iadd(baseFree);
    quoteInfo.balanceNative.iadd(quoteFree);

    // track the reserved amounts
    const reservedBase = I80F48.fromI64(
      oo.baseTokenTotal.sub(oo.baseTokenFree),
    );
    const reservedQuote = I80F48.fromI64(
      oo.quoteTokenTotal.sub(oo.quoteTokenFree),
    );

    return new Serum3Info(
      reservedBase,
      reservedQuote,
      baseIndex,
      quoteIndex,
      marketIndex,
    );
  }

  healthContribution(
    healthType: HealthType,
    tokenInfos: TokenInfo[],
    tokenMaxReserved: I80F48[],
    marketReserved: Serum3Reserved,
  ): I80F48 {
    if (
      marketReserved.allReservedAsBase.isZero() ||
      marketReserved.allReservedAsQuote.isZero()
    ) {
      return ZERO_I80F48();
    }

    const baseInfo = tokenInfos[this.baseIndex];
    const quoteInfo = tokenInfos[this.quoteIndex];
    const baseMaxReserved = tokenMaxReserved[this.baseIndex];
    const quoteMaxReserved = tokenMaxReserved[this.quoteIndex];

    // How much the health would increase if the reserved balance were applied to the passed
    // token info?
    const computeHealthEffect = function (
      tokenInfo: TokenInfo,
      tokenMaxReserved: I80F48,
      marketReserved: I80F48,
    ): I80F48 {
      // This balance includes all possible reserved funds from markets that relate to the
      // token, including this market itself: `tokenMaxReserved` is already included in `maxBalance`.
      const maxBalance = tokenInfo.balanceNative.add(tokenMaxReserved);

      // Assuming `reserved` was added to `max_balance` last (because that gives the smallest
      // health effects): how much did health change because of it?
      let assetPart, liabPart;
      if (maxBalance.gte(marketReserved)) {
        assetPart = marketReserved;
        liabPart = ZERO_I80F48();
      } else if (maxBalance.isNeg()) {
        assetPart = ZERO_I80F48();
        liabPart = marketReserved;
      } else {
        assetPart = maxBalance;
        liabPart = marketReserved.sub(maxBalance);
      }
      const assetWeight = tokenInfo.assetWeight(healthType);
      const liabWeight = tokenInfo.liabWeight(healthType);
      const assetPrice = tokenInfo.prices.asset(healthType);
      const liabPrice = tokenInfo.prices.liab(healthType);

      return assetWeight
        .mul(assetPart)
        .mul(assetPrice)
        .add(liabWeight.mul(liabPart).mul(liabPrice));
    };

    const healthBase = computeHealthEffect(
      baseInfo,
      baseMaxReserved,
      marketReserved.allReservedAsBase,
    );
    const healthQuote = computeHealthEffect(
      quoteInfo,
      quoteMaxReserved,
      marketReserved.allReservedAsQuote,
    );

    return healthBase.min(healthQuote);
  }

  toString(
    tokenInfos: TokenInfo[],
    tokenMaxReserved: I80F48[],
    marketReserved: Serum3Reserved,
  ): string {
    return `  marketIndex: ${this.marketIndex}, baseIndex: ${
      this.baseIndex
    }, quoteIndex: ${this.quoteIndex}, reservedBase: ${
      this.reservedBase
    }, reservedQuote: ${
      this.reservedQuote
    }, initHealth ${this.healthContribution(
      HealthType.init,
      tokenInfos,
      tokenMaxReserved,
      marketReserved,
    )}`;
  }
}

export class PerpInfo {
  constructor(
    public perpMarketIndex: number,
    public maintAssetWeight: I80F48,
    public initAssetWeight: I80F48,
    public maintLiabWeight: I80F48,
    public initLiabWeight: I80F48,
    public baseLotSize: BN,
    public baseLots: BN,
    public bidsBaseLots: BN,
    public asksBaseLots: BN,
    public quote: I80F48,
    public prices: Prices,
    public hasOpenOrders: boolean,
    public trustedMarket: boolean,
  ) {}

  static fromDto(dto: PerpInfoDto): PerpInfo {
    return new PerpInfo(
      dto.perpMarketIndex,
      I80F48.from(dto.maintAssetWeight),
      I80F48.from(dto.initAssetWeight),
      I80F48.from(dto.maintLiabWeight),
      I80F48.from(dto.initLiabWeight),
      dto.baseLotSize,
      dto.baseLots,
      dto.bidsBaseLots,
      dto.asksBaseLots,
      I80F48.from(dto.quote),
      new Prices(
        I80F48.from(dto.prices.oracle),
        I80F48.from(dto.prices.stable),
      ),
      dto.hasOpenOrders,
      dto.trustedMarket,
    );
  }

  static fromPerpPosition(
    perpMarket: PerpMarket,
    perpPosition: PerpPosition,
  ): PerpInfo {
    const baseLots = perpPosition.basePositionLots.add(
      perpPosition.takerBaseLots,
    );
    const unsettledFunding = perpPosition.getUnsettledFunding(perpMarket);

    const takerQuote = I80F48.fromI64(
      new BN(perpPosition.takerQuoteLots).mul(perpMarket.quoteLotSize),
    );
    const quoteCurrent = perpPosition.quotePositionNative
      .sub(unsettledFunding)
      .add(takerQuote);

    return new PerpInfo(
      perpMarket.perpMarketIndex,
      perpMarket.maintAssetWeight,
      perpMarket.initAssetWeight,
      perpMarket.maintLiabWeight,
      perpMarket.initLiabWeight,
      perpMarket.baseLotSize,
      baseLots,
      perpPosition.bidsBaseLots,
      perpPosition.asksBaseLots,
      quoteCurrent,
      new Prices(
        perpMarket.price,
        I80F48.fromNumber(perpMarket.stablePriceModel.stablePrice),
      ),
      perpPosition.hasOpenOrders(),
      perpMarket.trustedMarket,
    );
  }

  healthContribution(healthType: HealthType): I80F48 {
    return this.trustedMarket
      ? this.uncappedHealthContribution(healthType)
      : this.uncappedHealthContribution(healthType).min(ZERO_I80F48());
  }

  uncappedHealthContribution(healthType: HealthType): I80F48 {
    function orderExecutionCase(
      pi: PerpInfo,
      ordersBaseLots: BN,
      orderPrice: I80F48,
    ): I80F48 {
      const netBaseNative = I80F48.fromU64(
        pi.baseLots.add(ordersBaseLots).mul(pi.baseLotSize),
      );

      let weight, basePrice;
      if (healthType == HealthType.init) {
        if (netBaseNative.isNeg()) {
          weight = pi.initLiabWeight;
          basePrice = pi.prices.liab(healthType);
        } else {
          weight = pi.initAssetWeight;
          basePrice = pi.prices.asset(healthType);
        }
      } else {
        if (netBaseNative.isNeg()) {
          weight = pi.maintLiabWeight;
          basePrice = pi.prices.liab(healthType);
        } else {
          weight = pi.maintAssetWeight;
          basePrice = pi.prices.asset(healthType);
        }
      }

      // Total value of the order-execution adjusted base position
      const baseHealth = netBaseNative.mul(weight).mul(basePrice);

      const ordersBaseNative = I80F48.fromU64(
        ordersBaseLots.mul(pi.baseLotSize),
      );
      // The quote change from executing the bids/asks
      const orderQuote = ordersBaseNative.neg().mul(orderPrice);

      return baseHealth.add(orderQuote);
    }

    // What is worse: Executing all bids at oracle_price.liab, or executing all asks at oracle_price.asset?
    const bidsCase = orderExecutionCase(
      this,
      this.bidsBaseLots,
      this.prices.liab(healthType),
    );
    const asksCase = orderExecutionCase(
      this,
      this.asksBaseLots.neg(),
      this.prices.asset(healthType),
    );
    const worstCase = bidsCase.min(asksCase);

    return this.quote.add(worstCase);
  }

  static emptyFromPerpMarket(perpMarket: PerpMarket): PerpInfo {
    return new PerpInfo(
      perpMarket.perpMarketIndex,
      perpMarket.maintAssetWeight,
      perpMarket.initAssetWeight,
      perpMarket.maintLiabWeight,
      perpMarket.initLiabWeight,
      perpMarket.baseLotSize,
      new BN(0),
      new BN(0),
      new BN(0),
      ZERO_I80F48(),
      new Prices(
        perpMarket.price,
        I80F48.fromNumber(perpMarket.stablePriceModel.stablePrice),
      ),
      false,
      perpMarket.trustedMarket,
    );
  }

  toString(): string {
    return `  perpMarketIndex: ${this.perpMarketIndex}, base: ${
      this.baseLots
    }, quote: ${this.quote}, oraclePrice: ${
      this.prices.oracle
    }, uncapped health contribution ${this.uncappedHealthContribution(
      HealthType.init,
    )}`;
  }
}

export class HealthCacheDto {
  tokenInfos: TokenInfoDto[];
  serum3Infos: Serum3InfoDto[];
  perpInfos: PerpInfoDto[];
}
export class TokenInfoDto {
  tokenIndex: number;
  maintAssetWeight: I80F48Dto;
  initAssetWeight: I80F48Dto;
  maintLiabWeight: I80F48Dto;
  initLiabWeight: I80F48Dto;
  prices: { oracle: I80F48Dto; stable: I80F48Dto };
  balanceNative: I80F48Dto;

  constructor(
    tokenIndex: number,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    prices: { oracle: I80F48Dto; stable: I80F48Dto },
    balanceNative: I80F48Dto,
  ) {
    this.tokenIndex = tokenIndex;
    this.maintAssetWeight = maintAssetWeight;
    this.initAssetWeight = initAssetWeight;
    this.maintLiabWeight = maintLiabWeight;
    this.initLiabWeight = initLiabWeight;
    this.prices = prices;
    this.balanceNative = balanceNative;
  }
}

export class Serum3InfoDto {
  reservedBase: I80F48Dto;
  reservedQuote: I80F48Dto;
  baseIndex: number;
  quoteIndex: number;
  marketIndex: number;

  constructor(
    reservedBase: I80F48Dto,
    reservedQuote: I80F48Dto,
    baseIndex: number,
    quoteIndex: number,
  ) {
    this.reservedBase = reservedBase;
    this.reservedQuote = reservedQuote;
    this.baseIndex = baseIndex;
    this.quoteIndex = quoteIndex;
  }
}

export class PerpInfoDto {
  perpMarketIndex: number;
  maintAssetWeight: I80F48Dto;
  initAssetWeight: I80F48Dto;
  maintLiabWeight: I80F48Dto;
  initLiabWeight: I80F48Dto;
  public baseLotSize: BN;
  public baseLots: BN;
  public bidsBaseLots: BN;
  public asksBaseLots: BN;
  quote: I80F48Dto;
  prices: { oracle: I80F48Dto; stable: I80F48Dto };
  hasOpenOrders: boolean;
  trustedMarket: boolean;
}
