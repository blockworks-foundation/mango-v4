import { PublicKey } from '@solana/web3.js';
import _ from 'lodash';
import { Bank, BankForHealth } from './bank';
import { Group } from './group';
import {
  HUNDRED_I80F48,
  I80F48,
  I80F48Dto,
  MAX_I80F48,
  ONE_I80F48,
  ZERO_I80F48,
} from './I80F48';
import { HealthType } from './mangoAccount';
import { PerpMarket, PerpOrderSide } from './perp';
import { Serum3Market, Serum3Side } from './serum3';

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

  static fromDto(dto) {
    // console.log(
    JSON.stringify(
      dto,
      function replacer(k, v) {
        // console.log(k);
        console.log(v);
        // if (v instanceof BN) {
        // console.log(v);
        // return new I80F48(v).toNumber();
        // }
        // return v;
      },
      2,
    ),
      // );
      process.exit(0);
    return new HealthCache(
      dto.tokenInfos.map((dto) => TokenInfo.fromDto(dto)),
      dto.serum3Infos.map((dto) => Serum3Info.fromDto(dto)),
      dto.perpInfos.map((dto) => PerpInfo.fromDto(dto)),
    );
  }

  public health(healthType: HealthType): I80F48 {
    const health = ZERO_I80F48();
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      health.iadd(contrib);
    }
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
      );
      health.iadd(contrib);
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      health.iadd(contrib);
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
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
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
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
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
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
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

    if (liabs.isPos()) {
      return HUNDRED_I80F48().mul(assets.sub(liabs).div(liabs));
    } else {
      return MAX_I80F48();
    }
  }

  findTokenInfoIndex(tokenIndex: number): number {
    return this.tokenInfos.findIndex(
      (tokenInfo) => tokenInfo.tokenIndex == tokenIndex,
    );
  }

  getOrCreateTokenInfoIndex(bank: BankForHealth): number {
    const index = this.findTokenInfoIndex(bank.tokenIndex);
    if (index == -1) {
      this.tokenInfos.push(TokenInfo.emptyFromBank(bank));
    }
    return this.findTokenInfoIndex(bank.tokenIndex);
  }

  findSerum3InfoIndex(marketIndex: number): number {
    return this.serum3Infos.findIndex(
      (serum3Info) => serum3Info.marketIndex === marketIndex,
    );
  }

  getOrCreateSerum3InfoIndex(group: Group, serum3Market: Serum3Market): number {
    const index = this.findSerum3InfoIndex(serum3Market.marketIndex);
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );
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
    // todo change indices to types from numbers
    group: Group,
    serum3Market: Serum3Market,
    reservedBaseChange: I80F48,
    freeBaseChange: I80F48,
    reservedQuoteChange: I80F48,
    freeQuoteChange: I80F48,
  ) {
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );

    const baseEntryIndex = this.getOrCreateTokenInfoIndex(baseBank);
    const quoteEntryIndex = this.getOrCreateTokenInfoIndex(quoteBank);

    const baseEntry = this.tokenInfos[baseEntryIndex];
    const reservedAmount = reservedBaseChange.mul(baseEntry.oraclePrice);

    const quoteEntry = this.tokenInfos[quoteEntryIndex];
    reservedAmount.iadd(reservedQuoteChange.mul(quoteEntry.oraclePrice));

    // Apply it to the tokens
    baseEntry.serum3MaxReserved.iadd(reservedAmount);
    baseEntry.balance.iadd(freeBaseChange.mul(baseEntry.oraclePrice));
    quoteEntry.serum3MaxReserved.iadd(reservedAmount);
    quoteEntry.balance.iadd(freeQuoteChange.mul(quoteEntry.oraclePrice));

    // Apply it to the serum3 info
    const index = this.getOrCreateSerum3InfoIndex(group, serum3Market);
    const serum3Info = this.serum3Infos[index];
    serum3Info.reserved = serum3Info.reserved.add(reservedAmount);
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

  public static logHealthCache(debug: string, healthCache: HealthCache) {
    if (debug) console.log(debug);
    for (const token of healthCache.tokenInfos) {
      console.log(` ${token.toString()}`);
    }
    for (const serum3Info of healthCache.serum3Infos) {
      console.log(` ${serum3Info.toString(healthCache.tokenInfos)}`);
    }
    console.log(
      ` assets ${healthCache.assets(
        HealthType.init,
      )}, liabs ${healthCache.liabs(HealthType.init)}, `,
    );
    console.log(
      ` health(HealthType.init) ${healthCache.health(HealthType.init)}`,
    );
    console.log(
      ` healthRatio(HealthType.init) ${healthCache.healthRatio(
        HealthType.init,
      )}`,
    );
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
      if (!bank.price)
        throw new Error(
          `Oracle price not loaded for ${change.mintPk.toString()}`,
        );
      adjustedCache.tokenInfos[changeIndex].balance.iadd(
        change.nativeTokenAmount.mul(bank.price),
      );
    }
    // HealthCache.logHealthCache('afterChange', adjustedCache);
    return adjustedCache.healthRatio(healthType);
  }

  simHealthRatioWithSerum3BidChanges(
    group: Group,
    bidNativeQuoteAmount: I80F48,
    serum3Market: Serum3Market,
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    const adjustedCache: HealthCache = _.cloneDeep(this);
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );
    if (!quoteBank) {
      throw new Error(`No bank for index ${serum3Market.quoteTokenIndex}`);
    }
    const quoteIndex = adjustedCache.getOrCreateTokenInfoIndex(quoteBank);
    const quote = adjustedCache.tokenInfos[quoteIndex];

    // Move token balance to reserved funds in open orders,
    // essentially simulating a place order

    // Reduce token balance for quote
    adjustedCache.tokenInfos[quoteIndex].balance.isub(
      bidNativeQuoteAmount.mul(quote.oraclePrice),
    );

    // Increase reserved in Serum3Info for quote
    adjustedCache.adjustSerum3Reserved(
      group,
      serum3Market,
      ZERO_I80F48(),
      ZERO_I80F48(),
      bidNativeQuoteAmount,
      ZERO_I80F48(),
    );
    return adjustedCache.healthRatio(healthType);
  }

  simHealthRatioWithSerum3AskChanges(
    group: Group,
    askNativeBaseAmount: I80F48,
    serum3Market: Serum3Market,
    healthType: HealthType = HealthType.init,
  ): I80F48 {
    const adjustedCache: HealthCache = _.cloneDeep(this);
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    if (!baseBank) {
      throw new Error(`No bank for index ${serum3Market.quoteTokenIndex}`);
    }
    const baseIndex = adjustedCache.getOrCreateTokenInfoIndex(baseBank);
    const base = adjustedCache.tokenInfos[baseIndex];

    // Move token balance to reserved funds in open orders,
    // essentially simulating a place order

    // Reduce token balance for base
    adjustedCache.tokenInfos[baseIndex].balance.isub(
      askNativeBaseAmount.mul(base.oraclePrice),
    );

    // Increase reserved in Serum3Info for base
    adjustedCache.adjustSerum3Reserved(
      group,
      serum3Market,
      askNativeBaseAmount,
      ZERO_I80F48(),
      ZERO_I80F48(),
      ZERO_I80F48(),
    );
    return adjustedCache.healthRatio(healthType);
  }

  private static binaryApproximationSearch(
    left: I80F48,
    leftRatio: I80F48,
    right: I80F48,
    rightRatio: I80F48,
    targetRatio: I80F48,
    healthRatioAfterActionFn: (I80F48) => I80F48,
  ) {
    const maxIterations = 40;
    // TODO: make relative to health ratio decimals? Might be over engineering
    const targetError = I80F48.fromNumber(0.001);

    if (
      (leftRatio.sub(targetRatio).isPos() &&
        rightRatio.sub(targetRatio).isPos()) ||
      (leftRatio.sub(targetRatio).isNeg() &&
        rightRatio.sub(targetRatio).isNeg())
    ) {
      throw new Error(
        `internal error: left ${leftRatio.toNumber()}  and right ${rightRatio.toNumber()} don't contain the target value ${targetRatio.toNumber()}`,
      );
    }

    let newAmount;
    for (const key of Array(maxIterations).fill(0).keys()) {
      newAmount = left.add(right).mul(I80F48.fromNumber(0.5));
      const newAmountRatio = healthRatioAfterActionFn(newAmount);
      const error = newAmountRatio.sub(targetRatio);
      if (error.isPos() && error.lt(targetError)) {
        return newAmount;
      }
      if (newAmountRatio.gt(targetRatio) != rightRatio.gt(targetRatio)) {
        left = newAmount;
      } else {
        right = newAmount;
        rightRatio = newAmountRatio;
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
    minRatio: I80F48,
    priceFactor: I80F48,
  ): I80F48 {
    if (
      !sourceBank.price ||
      sourceBank.price.lte(ZERO_I80F48()) ||
      !targetBank.price ||
      targetBank.price.lte(ZERO_I80F48())
    ) {
      return ZERO_I80F48();
    }

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
    const initialHealth = this.health(HealthType.init);
    if (initialRatio.lte(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    // If the price is sufficiently good, then health will just increase from swapping:
    // once we've swapped enough, swapping x reduces health by x * source_liab_weight and
    // increases it by x * target_asset_weight * price_factor.
    const finalHealthSlope = sourceBank.initLiabWeight
      .neg()
      .add(targetBank.initAssetWeight.mul(priceFactor));
    if (finalHealthSlope.gte(ZERO_I80F48())) {
      return MAX_I80F48();
    }

    const healthCacheClone: HealthCache = _.cloneDeep(this);
    const sourceIndex = healthCacheClone.getOrCreateTokenInfoIndex(sourceBank);
    const targetIndex = healthCacheClone.getOrCreateTokenInfoIndex(targetBank);
    const source = healthCacheClone.tokenInfos[sourceIndex];
    const target = healthCacheClone.tokenInfos[targetIndex];

    // There are two key slope changes: Assume source.balance > 0 and target.balance < 0. Then
    // initially health ratio goes up. When one of balances flips sign, the health ratio slope
    // may be positive or negative for a bit, until both balances have flipped and the slope is
    // negative.
    // The maximum will be at one of these points (ignoring serum3 effects).

    function cacheAfterSwap(amount: I80F48) {
      const adjustedCache: HealthCache = _.cloneDeep(healthCacheClone);
      // HealthCache.logHealthCache('beforeSwap', adjustedCache);
      adjustedCache.tokenInfos[sourceIndex].balance.isub(amount);
      adjustedCache.tokenInfos[targetIndex].balance.iadd(
        amount.mul(priceFactor),
      );
      // HealthCache.logHealthCache('afterSwap', adjustedCache);
      return adjustedCache;
    }

    function healthRatioAfterSwap(amount: I80F48): I80F48 {
      return cacheAfterSwap(amount).healthRatio(HealthType.init);
    }

    // There are two key slope changes: Assume source.balance > 0 and target.balance < 0.
    // When these values flip sign, the health slope decreases, but could still be positive.
    // After point1 it's definitely negative (due to finalHealthSlope check above).
    // The maximum health ratio will be at 0 or at one of these points (ignoring serum3 effects).
    const sourceForZeroTargetBalance = target.balance.neg().div(priceFactor);
    const point0Amount = source.balance
      .min(sourceForZeroTargetBalance)
      .max(ZERO_I80F48());
    const point1Amount = source.balance
      .max(sourceForZeroTargetBalance)
      .max(ZERO_I80F48());
    const cache0 = cacheAfterSwap(point0Amount);
    const point0Ratio = cache0.healthRatio(HealthType.init);
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
      if (point1Health.lte(ZERO_I80F48())) {
        return ZERO_I80F48();
      }
      const zeroHealthAmount = point1Amount.sub(
        point1Health.div(finalHealthSlope),
      );
      const zeroHealthRatio = healthRatioAfterSwap(zeroHealthAmount);
      amount = HealthCache.binaryApproximationSearch(
        point1Amount,
        point1Ratio,
        zeroHealthAmount,
        zeroHealthRatio,
        minRatio,
        healthRatioAfterSwap,
      );
    } else if (point0Ratio.gte(minRatio)) {
      // Must be between point0Amount and point1Amount.
      amount = HealthCache.binaryApproximationSearch(
        point0Amount,
        point0Ratio,
        point1Amount,
        point1Ratio,
        minRatio,
        healthRatioAfterSwap,
      );
    } else {
      // Must be between 0 and point0_amount
      amount = HealthCache.binaryApproximationSearch(
        ZERO_I80F48(),
        initialRatio,
        point0Amount,
        point0Ratio,
        minRatio,
        healthRatioAfterSwap,
      );
    }

    return amount.div(source.oraclePrice);
  }

  getMaxSerum3OrderForHealthRatio(
    group: Group,
    serum3Market: Serum3Market,
    side: Serum3Side,
    minRatio: I80F48,
  ) {
    const baseBank = group.getFirstBankByTokenIndex(
      serum3Market.baseTokenIndex,
    );
    if (!baseBank) {
      throw new Error(`No bank for index ${serum3Market.baseTokenIndex}`);
    }
    const quoteBank = group.getFirstBankByTokenIndex(
      serum3Market.quoteTokenIndex,
    );
    if (!quoteBank) {
      throw new Error(`No bank for index ${serum3Market.quoteTokenIndex}`);
    }

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
      const quoteBorrows = quote.balance.lt(ZERO_I80F48())
        ? quote.balance.abs()
        : ZERO_I80F48();
      const max = base.balance.max(quoteBorrows);
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
      const baseBorrows = base.balance.lt(ZERO_I80F48())
        ? base.balance.abs()
        : ZERO_I80F48();
      const max = quote.balance.max(baseBorrows);
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
    const zeroAmountHealth = cache.health(HealthType.init);
    const zeroAmountRatio = cache.healthRatio(HealthType.init);

    function cacheAfterPlacingOrder(amount: I80F48) {
      const adjustedCache: HealthCache = _.cloneDeep(healthCacheClone);

      side === Serum3Side.ask
        ? adjustedCache.tokenInfos[baseIndex].balance.isub(amount)
        : adjustedCache.tokenInfos[quoteIndex].balance.isub(amount);

      adjustedCache.adjustSerum3Reserved(
        group,
        serum3Market,
        side === Serum3Side.ask ? amount.div(base.oraclePrice) : ZERO_I80F48(),
        ZERO_I80F48(),
        side === Serum3Side.bid ? amount.div(quote.oraclePrice) : ZERO_I80F48(),
        ZERO_I80F48(),
      );

      return adjustedCache;
    }

    function healthRatioAfterPlacingOrder(amount: I80F48): I80F48 {
      return cacheAfterPlacingOrder(amount).healthRatio(HealthType.init);
    }

    const amount = HealthCache.binaryApproximationSearch(
      initialAmount,
      initialRatio,
      zeroAmount,
      zeroAmountRatio,
      minRatio,
      healthRatioAfterPlacingOrder,
    );

    // If its a bid then the reserved fund and potential loan is in quote,
    // If its a ask then the reserved fund and potential loan is in base,
    // also keep some buffer for fees, use taker fees for worst case simulation.
    return side === Serum3Side.bid
      ? amount
          .div(quote.oraclePrice)
          .div(ONE_I80F48().add(baseBank.loanOriginationFeeRate))
          .div(ONE_I80F48().add(I80F48.fromNumber(group.getFeeRate(false))))
      : amount
          .div(base.oraclePrice)
          .div(ONE_I80F48().add(quoteBank.loanOriginationFeeRate))
          .div(ONE_I80F48().add(I80F48.fromNumber(group.getFeeRate(false))));
  }

  getMaxPerpForHealthRatio(
    perpMarket: PerpMarket,
    side: PerpOrderSide,
    minRatio: I80F48,
    price: I80F48,
  ): I80F48 {
    const healthCacheClone: HealthCache = _.cloneDeep(this);

    const initialRatio = this.healthRatio(HealthType.init);
    if (initialRatio.lt(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    const direction = side == PerpOrderSide.bid ? 1 : -1;

    const perpInfoIndex = this.getOrCreatePerpInfoIndex(perpMarket);
    const perpInfo = this.perpInfos[perpInfoIndex];
    const oraclePrice = perpInfo.oraclePrice;
    const baseLotSize = I80F48.fromString(perpMarket.baseLotSize.toString());

    // If the price is sufficiently good then health will just increase from trading
    const finalHealthSlope =
      direction == 1
        ? perpInfo.initAssetWeight.mul(oraclePrice).sub(price)
        : price.sub(perpInfo.initLiabWeight.mul(oraclePrice));
    if (finalHealthSlope.gte(ZERO_I80F48())) {
      return MAX_I80F48();
    }

    function cacheAfterTrade(baseLots: I80F48): HealthCache {
      const adjustedCache: HealthCache = _.cloneDeep(healthCacheClone);
      const d = I80F48.fromNumber(direction);
      adjustedCache.perpInfos[perpInfoIndex].base.iadd(
        d.mul(baseLots.mul(baseLotSize.mul(oraclePrice))),
      );
      adjustedCache.perpInfos[perpInfoIndex].quote.isub(
        d.mul(baseLots.mul(baseLotSize.mul(price))),
      );
      return adjustedCache;
    }

    function healthAfterTrade(baseLots: I80F48): I80F48 {
      return cacheAfterTrade(baseLots).health(HealthType.init);
    }
    function healthRatioAfterTrade(baseLots: I80F48): I80F48 {
      return cacheAfterTrade(baseLots).healthRatio(HealthType.init);
    }

    const initialBaseLots = perpInfo.base
      .div(perpInfo.oraclePrice)
      .div(baseLotSize);

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
      const case1StartHealth = healthAfterTrade(case1Start);
      if (case1StartHealth.lte(ZERO_I80F48())) {
        return ZERO_I80F48();
      }
      const zeroHealthAmount = case1Start.sub(
        case1StartHealth.div(finalHealthSlope).div(baseLotSize),
      );
      const zeroHealthRatio = healthRatioAfterTrade(zeroHealthAmount);
      baseLots = HealthCache.binaryApproximationSearch(
        case1Start,
        case1StartRatio,
        zeroHealthAmount,
        zeroHealthRatio,
        minRatio,
        healthRatioAfterTrade,
      );
    } else {
      // Between 0 and case1Start
      baseLots = HealthCache.binaryApproximationSearch(
        ZERO_I80F48(),
        initialRatio,
        case1Start,
        case1StartRatio,
        minRatio,
        healthRatioAfterTrade,
      );
    }

    return baseLots.floor();
  }
}

export class TokenInfo {
  constructor(
    public tokenIndex: number,
    public maintAssetWeight: I80F48,
    public initAssetWeight: I80F48,
    public maintLiabWeight: I80F48,
    public initLiabWeight: I80F48,
    // native/native
    public oraclePrice: I80F48,
    // in health-reference-token native units
    public balance: I80F48,
    // in health-reference-token native units
    public serum3MaxReserved: I80F48,
  ) {}

  static fromDto(dto: TokenInfoDto): TokenInfo {
    return new TokenInfo(
      dto.tokenIndex,
      I80F48.from(dto.maintAssetWeight),
      I80F48.from(dto.initAssetWeight),
      I80F48.from(dto.maintLiabWeight),
      I80F48.from(dto.initLiabWeight),
      I80F48.from(dto.oraclePrice),
      I80F48.from(dto.balance),
      I80F48.from(dto.serum3MaxReserved),
    );
  }

  static emptyFromBank(bank: BankForHealth): TokenInfo {
    if (!bank.price)
      throw new Error(
        `Failed to create TokenInfo. Bank price unavailable for bank with tokenIndex ${bank.tokenIndex}`,
      );
    return new TokenInfo(
      bank.tokenIndex,
      bank.maintAssetWeight,
      bank.initAssetWeight,
      bank.maintLiabWeight,
      bank.initLiabWeight,
      bank.price,
      ZERO_I80F48(),
      ZERO_I80F48(),
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
    return (
      this.balance.isNeg()
        ? this.liabWeight(healthType)
        : this.assetWeight(healthType)
    ).mul(this.balance);
  }

  toString() {
    return `  tokenIndex: ${this.tokenIndex}, balance: ${
      this.balance
    }, serum3MaxReserved: ${
      this.serum3MaxReserved
    }, initHealth ${this.healthContribution(HealthType.init)}`;
  }
}

export class Serum3Info {
  constructor(
    public reserved: I80F48,
    public baseIndex: number,
    public quoteIndex: number,
    public marketIndex: number,
  ) {}

  static fromDto(dto: Serum3InfoDto) {
    return new Serum3Info(
      I80F48.from(dto.reserved),
      dto.baseIndex,
      dto.quoteIndex,
      dto.marketIndex,
    );
  }

  static emptyFromSerum3Market(
    serum3Market: Serum3Market,
    baseEntryIndex: number,
    quoteEntryIndex: number,
  ) {
    return new Serum3Info(
      ZERO_I80F48(),
      baseEntryIndex,
      quoteEntryIndex,
      serum3Market.marketIndex,
    );
  }

  healthContribution(healthType: HealthType, tokenInfos: TokenInfo[]): I80F48 {
    const baseInfo = tokenInfos[this.baseIndex];
    const quoteInfo = tokenInfos[this.quoteIndex];
    const reserved = this.reserved;

    if (reserved.isZero()) {
      return ZERO_I80F48();
    }

    // How much the health would increase if the reserved balance were applied to the passed
    // token info?
    const computeHealthEffect = function (tokenInfo: TokenInfo) {
      // This balance includes all possible reserved funds from markets that relate to the
      // token, including this market itself: `reserved` is already included in `max_balance`.
      const maxBalance = tokenInfo.balance.add(tokenInfo.serum3MaxReserved);

      // Assuming `reserved` was added to `max_balance` last (because that gives the smallest
      // health effects): how much did health change because of it?
      let assetPart, liabPart;
      if (maxBalance.gte(reserved)) {
        assetPart = reserved;
        liabPart = ZERO_I80F48();
      } else if (maxBalance.isNeg()) {
        assetPart = ZERO_I80F48();
        liabPart = reserved;
      } else {
        assetPart = maxBalance;
        liabPart = reserved.sub(maxBalance);
      }
      const assetWeight = tokenInfo.assetWeight(healthType);
      const liabWeight = tokenInfo.liabWeight(healthType);
      return assetWeight.mul(assetPart).add(liabWeight.mul(liabPart));
    };

    const reservedAsBase = computeHealthEffect(baseInfo);
    const reservedAsQuote = computeHealthEffect(quoteInfo);
    return reservedAsBase.min(reservedAsQuote);
  }

  toString(tokenInfos: TokenInfo[]) {
    return `  marketIndex: ${this.marketIndex}, baseIndex: ${
      this.baseIndex
    }, quoteIndex: ${this.quoteIndex}, reserved: ${
      this.reserved
    }, initHealth ${this.healthContribution(HealthType.init, tokenInfos)}`;
  }
}

export class PerpInfo {
  constructor(
    public perpMarketIndex: number,
    public maintAssetWeight: I80F48,
    public initAssetWeight: I80F48,
    public maintLiabWeight: I80F48,
    public initLiabWeight: I80F48,
    // in health-reference-token native units, needs scaling by asset/liab
    public base: I80F48,
    // in health-reference-token native units, no asset/liab factor needed
    public quote: I80F48,
    public oraclePrice: I80F48,
    public hasOpenOrders: boolean,
  ) {}

  static fromDto(dto: PerpInfoDto) {
    return new PerpInfo(
      dto.perpMarketIndex,
      I80F48.from(dto.maintAssetWeight),
      I80F48.from(dto.initAssetWeight),
      I80F48.from(dto.maintLiabWeight),
      I80F48.from(dto.initLiabWeight),
      I80F48.from(dto.base),
      I80F48.from(dto.quote),
      I80F48.from(dto.oraclePrice),
      dto.hasOpenOrders,
    );
  }

  healthContribution(healthType: HealthType): I80F48 {
    let weight;
    if (healthType == HealthType.init && this.base.isNeg()) {
      weight = this.initLiabWeight;
    } else if (healthType == HealthType.init && !this.base.isNeg()) {
      weight = this.initAssetWeight;
    }
    if (healthType == HealthType.maint && this.base.isNeg()) {
      weight = this.maintLiabWeight;
    }
    if (healthType == HealthType.maint && !this.base.isNeg()) {
      weight = this.maintAssetWeight;
    }

    // FUTURE: Allow v3-style "reliable" markets where we can return
    // `self.quote + weight * self.base` here
    return this.quote.add(weight.mul(this.base)).min(ZERO_I80F48());
  }

  static emptyFromPerpMarket(perpMarket: PerpMarket): PerpInfo {
    if (!perpMarket.price)
      throw new Error(
        `Failed to create PerpInfo. Oracle price unavailable. ${perpMarket.oracle.toString()}`,
      );
    return new PerpInfo(
      perpMarket.perpMarketIndex,
      perpMarket.maintAssetWeight,
      perpMarket.initAssetWeight,
      perpMarket.maintLiabWeight,
      perpMarket.initLiabWeight,
      ZERO_I80F48(),
      ZERO_I80F48(),
      I80F48.fromNumber(perpMarket.price),
      false,
    );
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
  oraclePrice: I80F48Dto; // native/native
  // in health-reference-token native units
  balance: I80F48Dto;
  // in health-reference-token native units
  serum3MaxReserved: I80F48Dto;

  constructor(
    tokenIndex: number,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    oraclePrice: I80F48Dto,
    balance: I80F48Dto,
    serum3MaxReserved: I80F48Dto,
  ) {
    this.tokenIndex = tokenIndex;
    this.maintAssetWeight = maintAssetWeight;
    this.initAssetWeight = initAssetWeight;
    this.maintLiabWeight = maintLiabWeight;
    this.initLiabWeight = initLiabWeight;
    this.oraclePrice = oraclePrice;
    this.balance = balance;
    this.serum3MaxReserved = serum3MaxReserved;
  }
}

export class Serum3InfoDto {
  reserved: I80F48Dto;
  baseIndex: number;
  quoteIndex: number;
  marketIndex: number;

  constructor(reserved: I80F48Dto, baseIndex: number, quoteIndex: number) {
    this.reserved = reserved;
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
  // in health-reference-token native units, needs scaling by asset/liab
  base: I80F48Dto;
  // in health-reference-token native units, no asset/liab factor needed
  quote: I80F48Dto;
  oraclePrice: I80F48Dto;
  hasOpenOrders: boolean;
}
