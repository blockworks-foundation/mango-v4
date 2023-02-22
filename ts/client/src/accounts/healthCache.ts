import { BN } from '@coral-xyz/anchor';
import { OpenOrders } from '@project-serum/serum';
import { PublicKey } from '@solana/web3.js';
import cloneDeep from 'lodash/cloneDeep';
import {
  HUNDRED_I80F48,
  I80F48,
  I80F48Dto,
  MAX_I80F48,
  ONE_I80F48,
  ZERO_I80F48,
} from '../numbers/I80F48';
import { toNativeI80F48ForQuote } from '../utils';
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
      const positiveContrib = perpInfo
        .healthContribution(HealthType.maint)
        .max(ZERO_I80F48());
      // console.log(` - pi ${positiveContrib}`);
      health.iadd(positiveContrib);
    }
    return health;
  }

  // An undefined HealthType will use an asset and liab weight of 1
  public assets(healthType?: HealthType): I80F48 {
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

  // An undefined HealthType will use an asset and liab weight of 1
  public liabs(healthType?: HealthType): I80F48 {
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
      // console.log(` - ti contrib ${contrib.toLocaleString()}`);
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
      // console.log(` - si contrib ${contrib.toLocaleString()}`);
      if (contrib.isPos()) {
        assets.iadd(contrib);
      } else {
        liabs.isub(contrib);
      }
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      // console.log(` - pi contrib ${contrib.toLocaleString()}`);
      if (contrib.isPos()) {
        assets.iadd(contrib);
      } else {
        liabs.isub(contrib);
      }
    }

    // console.log(
    //   ` - assets ${assets.toLocaleString()}, liabs ${liabs.toLocaleString()}`,
    // );

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
    const adjustedCache: HealthCache = cloneDeep(this);
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
    const adjustedCache: HealthCache = cloneDeep(this);
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
    const adjustedCache: HealthCache = cloneDeep(this);
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
    const clonedHealthCache: HealthCache = cloneDeep(this);
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
    // console.log(`scanRightUntilLessThan, start ${start.toLocaleString()}`);
    for (const key of Array(maxIterations).fill(0).keys()) {
      const value = fun(current);
      if (value.lt(target)) {
        return current;
      }
      // console.log(
      //   ` - current ${current.toLocaleString()}, value ${value.toLocaleString()}, target ${target.toLocaleString()}`,
      // );
      current = current.max(ONE_I80F48()).mul(I80F48.fromNumber(2));
    }
    throw new Error('Could not find amount that led to health ratio <=0');
  }

  /// This is not a generic function. It assumes there is a unique maximum between left and right.
  private static findMaximum(
    left: I80F48,
    right: I80F48,
    minStep: I80F48,
    fun: (I80F48) => I80F48,
  ): I80F48[] {
    const half = I80F48.fromNumber(0.5);
    let mid = half.mul(left.add(right));
    let leftValue = fun(left);
    let rightValue = fun(right);
    let midValue = fun(mid);
    while (right.sub(left).gt(minStep)) {
      if (leftValue.gte(midValue)) {
        // max must be between left and mid
        right = mid;
        rightValue = midValue;
        mid = half.mul(left.add(mid));
        midValue = fun(mid);
      } else if (midValue.lte(rightValue)) {
        // max must be between mid and right
        left = mid;
        leftValue = midValue;
        mid = half.mul(mid.add(right));
        midValue = fun(mid);
      } else {
        // mid is larger than both left and right, max could be on either side
        const leftmid = half.mul(left.add(mid));
        const leftMidValue = fun(leftmid);
        if (leftMidValue.gte(midValue)) {
          // max between left and mid
          right = mid;
          rightValue = midValue;
          mid = leftmid;
          midValue = leftMidValue;
          continue;
        }

        const rightmid = half.mul(mid.add(right));
        const rightMidValue = fun(rightmid);
        if (rightMidValue.gte(midValue)) {
          // max between mid and right
          left = mid;
          leftValue = midValue;
          mid = rightmid;
          midValue = rightMidValue;
          continue;
        }

        // max between leftmid and rightmid
        left = leftmid;
        leftValue = leftMidValue;
        right = rightmid;
        rightValue = rightMidValue;
      }
    }

    if (leftValue.gte(midValue)) {
      return [left, leftValue];
    } else if (midValue.gte(rightValue)) {
      return [mid, midValue];
    } else {
      return [right, rightValue];
    }
  }

  private static binaryApproximationSearch(
    left: I80F48,
    leftValue: I80F48,
    right: I80F48,
    targetValue: I80F48,
    minStep: I80F48,
    fun: (I80F48) => I80F48,
  ): I80F48 {
    const maxIterations = 50;
    const targetError = I80F48.fromNumber(0.1);
    const rightValue = fun(right);

    // console.log(
    //   ` - binaryApproximationSearch left ${left.toLocaleString()}, leftValue ${leftValue.toLocaleString()}, right ${right.toLocaleString()}, rightValue ${rightValue.toLocaleString()}, targetValue ${targetValue.toLocaleString()}`,
    // );

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

    let newAmount, newAmountValue;
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    for (const key of Array(maxIterations).fill(0).keys()) {
      if (right.sub(left).abs().lt(minStep)) {
        return left;
      }
      newAmount = left.add(right).mul(I80F48.fromNumber(0.5));
      newAmountValue = fun(newAmount);
      // console.log(
      //   `   - left ${left.toLocaleString()}, right ${right.toLocaleString()}, newAmount ${newAmount.toLocaleString()}, newAmountValue ${newAmountValue.toLocaleString()}, targetValue ${targetValue.toLocaleString()}`,
      // );
      const error = newAmountValue.sub(targetValue);
      if (error.isPos() && error.lt(targetError)) {
        return newAmount;
      }
      if (newAmountValue.gt(targetValue) != rightValue.gt(targetValue)) {
        left = newAmount;
      } else {
        right = newAmount;
      }
    }

    console.error(
      `Unable to get targetValue within ${maxIterations} iterations, newAmount ${newAmount}, newAmountValue ${newAmountValue}, target ${targetValue}`,
    );

    return newAmount;
  }

  getMaxSwapSource(
    sourceBank: BankForHealth,
    targetBank: BankForHealth,
    price: I80F48,
  ): I80F48 {
    const health = this.health(HealthType.init);
    if (health.isNeg()) {
      return this.getMaxSwapSourceForHealth(
        sourceBank,
        targetBank,
        price,
        toNativeI80F48ForQuote(1), // target 1 ui usd worth health
      );
    }
    return this.getMaxSwapSourceForHealthRatio(
      sourceBank,
      targetBank,
      price,
      I80F48.fromNumber(2), // target 2% health
    );
  }

  getMaxSwapSourceForHealthRatio(
    sourceBank: BankForHealth,
    targetBank: BankForHealth,
    price: I80F48,
    minRatio: I80F48,
  ): I80F48 {
    return this.getMaxSwapSourceForHealthFn(
      sourceBank,
      targetBank,
      price,
      minRatio,
      function (hc: HealthCache): I80F48 {
        return hc.healthRatio(HealthType.init);
      },
    );
  }

  getMaxSwapSourceForHealth(
    sourceBank: BankForHealth,
    targetBank: BankForHealth,
    price: I80F48,
    minHealth: I80F48,
  ): I80F48 {
    return this.getMaxSwapSourceForHealthFn(
      sourceBank,
      targetBank,
      price,
      minHealth,
      function (hc: HealthCache): I80F48 {
        return hc.health(HealthType.init);
      },
    );
  }

  getMaxSwapSourceForHealthFn(
    sourceBank: BankForHealth,
    targetBank: BankForHealth,
    price: I80F48,
    minFnValue: I80F48,
    targetFn: (cache) => I80F48,
  ): I80F48 {
    if (
      sourceBank.initLiabWeight
        .sub(targetBank.initAssetWeight)
        .abs()
        .lte(ZERO_I80F48())
    ) {
      return ZERO_I80F48();
    }

    // The health and health_ratio are nonlinear based on swap amount.
    // For large swap amounts the slope is guaranteed to be negative, but small amounts
    // can have positive slope (e.g. using source deposits to pay back target borrows).
    //
    // That means:
    // - even if the initial value is < minRatio it can be useful to swap to *increase* health
    // - even if initial value is < 0, swapping can increase health (maybe above 0)
    // - be careful about finding the minFnValue: the function isn't convex

    const initialRatio = this.healthRatio(HealthType.init);
    // eslint-disable-next-line @typescript-eslint/no-unused-vars

    const healthCacheClone: HealthCache = cloneDeep(this);
    const sourceIndex = healthCacheClone.getOrCreateTokenInfoIndex(sourceBank);
    const targetIndex = healthCacheClone.getOrCreateTokenInfoIndex(targetBank);

    const source = healthCacheClone.tokenInfos[sourceIndex];
    const target = healthCacheClone.tokenInfos[targetIndex];

    const res = healthCacheClone.computeSerum3Reservations(HealthType.init);
    const sourceReserved = res.tokenMaxReserved[sourceIndex];
    const targetReserved = res.tokenMaxReserved[targetIndex];

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
      const adjustedCache: HealthCache = cloneDeep(healthCacheClone);
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

    function fnValueAfterSwap(amount: I80F48): I80F48 {
      return targetFn(cacheAfterSwap(amount));
    }

    // The function we're looking at has a unique maximum.
    //
    // If we discount serum3 reservations, there are two key slope changes:
    // Assume source.balance > 0 and target.balance < 0.
    // When these values flip sign, the health slope decreases, but could still be positive.
    //
    // The first thing we do is to find this maximum.

    // The largest amount that the maximum could be at
    const rightmost = source.balanceNative
      .abs()
      .add(sourceReserved)
      .max(target.balanceNative.abs().add(targetReserved).div(price));
    const [amountForMaxValue, maxValue] = HealthCache.findMaximum(
      ZERO_I80F48(),
      rightmost,
      I80F48.fromNumber(0.1),
      fnValueAfterSwap,
    );

    if (maxValue.lte(minFnValue)) {
      // We cannot reach min_ratio, just return the max
      return amountForMaxValue;
    }

    let amount: I80F48;

    // Now max_value is bigger than minFnValue, the target amount must be >amountForMaxValue.
    // Search to the right of amountForMaxValue: but how far?
    // Use a simple estimation for the amount that would lead to zero health:
    //           health
    //              - source_liab_weight * source_liab_price * a
    //              + target_asset_weight * target_asset_price * price * a = 0.
    // where a is the source token native amount.
    // Note that this is just an estimate. Swapping can increase the amount that serum3
    // reserved contributions offset, moving the actual zero point further to the right.
    const healthAtMaxValue = cacheAfterSwap(amountForMaxValue).health(
      HealthType.init,
    );
    if (healthAtMaxValue.lte(ZERO_I80F48())) {
      return ZERO_I80F48();
    }
    const zeroHealthEstimate = amountForMaxValue.sub(
      healthAtMaxValue.div(finalHealthSlope),
    );
    const rightBound = HealthCache.scanRightUntilLessThan(
      zeroHealthEstimate,
      minFnValue,
      fnValueAfterSwap,
    );
    if (rightBound.eq(zeroHealthEstimate)) {
      amount = HealthCache.binaryApproximationSearch(
        amountForMaxValue,
        maxValue,
        rightBound,
        minFnValue,
        I80F48.fromNumber(0.1),
        fnValueAfterSwap,
      );
    } else {
      // Must be between 0 and point0_amount
      amount = HealthCache.binaryApproximationSearch(
        zeroHealthEstimate,
        fnValueAfterSwap(zeroHealthEstimate),
        rightBound,
        minFnValue,
        I80F48.fromNumber(0.1),
        fnValueAfterSwap,
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
    const healthCacheClone: HealthCache = cloneDeep(this);

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

    // console.log(`getMaxSerum3OrderForHealthRatio`);

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
      // console.log(` - quoteBorrows ${quoteBorrows.toLocaleString()}`);
      // console.log(` - max ${max.toLocaleString()}`);
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
      // console.log(` - baseBorrows ${baseBorrows.toLocaleString()}`);
      // console.log(` - max ${max.toLocaleString()}`);
    }

    const cache = cacheAfterPlacingOrder(zeroAmount);
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const zeroAmountHealth = cache.health(HealthType.init);
    const zeroAmountRatio = cache.healthRatio(HealthType.init);

    // console.log(` - zeroAmount ${zeroAmount.toLocaleString()}`);
    // console.log(` - zeroAmountHealth ${zeroAmountHealth.toLocaleString()}`);
    // console.log(` - zeroAmountRatio ${zeroAmountRatio.toLocaleString()}`);

    function cacheAfterPlacingOrder(amount: I80F48): HealthCache {
      const adjustedCache: HealthCache = cloneDeep(healthCacheClone);
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
    const healthCacheClone: HealthCache = cloneDeep(this);

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
        ? perpInfo.initBaseAssetWeight
            .mul(prices.asset(HealthType.init))
            .sub(price)
        : price.sub(
            perpInfo.initBaseLiabWeight.mul(prices.liab(HealthType.init)),
          );
    if (finalHealthSlope.gte(ZERO_I80F48())) {
      return MAX_I80F48();
    }

    function cacheAfterTrade(baseLots: BN): HealthCache {
      const adjustedCache: HealthCache = cloneDeep(healthCacheClone);
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
      // traded base lot (finalHealthSlope).
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
        .add(perpInfo.unweightedHealthContribution(HealthType.init));

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

  public liab(healthType: HealthType | undefined): I80F48 {
    if (
      healthType === HealthType.maint ||
      healthType === HealthType.liquidationEnd ||
      healthType === undefined
    ) {
      return this.oracle;
    }
    return this.oracle.max(this.stable);
  }

  public asset(healthType: HealthType | undefined): I80F48 {
    if (
      healthType === HealthType.maint ||
      healthType === HealthType.liquidationEnd ||
      healthType === undefined
    ) {
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
    public initScaledAssetWeight: I80F48,
    public maintLiabWeight: I80F48,
    public initLiabWeight: I80F48,
    public initScaledLiabWeight: I80F48,
    public prices: Prices,
    public balanceNative: I80F48,
  ) {}

  static fromDto(dto: TokenInfoDto): TokenInfo {
    return new TokenInfo(
      dto.tokenIndex as TokenIndex,
      I80F48.from(dto.maintAssetWeight),
      I80F48.from(dto.initAssetWeight),
      I80F48.from(dto.initScaledAssetWeight),
      I80F48.from(dto.maintLiabWeight),
      I80F48.from(dto.initLiabWeight),
      I80F48.from(dto.initScaledLiabWeight),
      new Prices(
        I80F48.from(dto.prices.oracle),
        I80F48.from(dto.prices.stable),
      ),
      I80F48.from(dto.balanceNative),
    );
  }

  static fromBank(bank: BankForHealth, nativeBalance?: I80F48): TokenInfo {
    const p = new Prices(
      bank.price,
      I80F48.fromNumber(bank.stablePriceModel.stablePrice),
    );
    // Use the liab price for computing weight scaling, because it's pessimistic and
    // causes the most unfavorable scaling.
    const liabPrice = p.liab(HealthType.init);
    return new TokenInfo(
      bank.tokenIndex,
      bank.maintAssetWeight,
      bank.initAssetWeight,
      bank.scaledInitAssetWeight(liabPrice),
      bank.maintLiabWeight,
      bank.initLiabWeight,
      bank.scaledInitLiabWeight(liabPrice),
      p,
      nativeBalance ? nativeBalance : ZERO_I80F48(),
    );
  }

  assetWeight(healthType: HealthType): I80F48 {
    if (healthType == HealthType.init) {
      return this.initScaledAssetWeight;
    } else if (healthType == HealthType.liquidationEnd) {
      return this.initAssetWeight;
    }
    // healthType == HealthType.maint
    return this.maintAssetWeight;
  }

  liabWeight(healthType: HealthType): I80F48 {
    if (healthType == HealthType.init) {
      return this.initScaledLiabWeight;
    } else if (healthType == HealthType.liquidationEnd) {
      return this.initLiabWeight;
    }
    // healthType == HealthType.maint
    return this.maintLiabWeight;
  }

  healthContribution(healthType?: HealthType): I80F48 {
    let weight, price;
    if (healthType === undefined) {
      return this.balanceNative.mul(this.prices.oracle);
    }
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

  // An undefined HealthType will use an asset and liab weight of 1
  healthContribution(
    healthType: HealthType | undefined,
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

      if (healthType === undefined) {
        return assetPart
          .mul(tokenInfo.prices.oracle)
          .add(liabPart.mul(tokenInfo.prices.oracle));
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

    // console.log(` - healthBase ${healthBase.toLocaleString()}`);
    // console.log(` - healthQuote ${healthQuote.toLocaleString()}`);

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
    public maintBaseAssetWeight: I80F48,
    public initBaseAssetWeight: I80F48,
    public maintBaseLiabWeight: I80F48,
    public initBaseLiabWeight: I80F48,
    public maintOverallAssetWeight: I80F48,
    public initOverallAssetWeight: I80F48,
    public baseLotSize: BN,
    public baseLots: BN,
    public bidsBaseLots: BN,
    public asksBaseLots: BN,
    public quote: I80F48,
    public prices: Prices,
    public hasOpenOrders: boolean,
  ) {}

  static fromDto(dto: PerpInfoDto): PerpInfo {
    return new PerpInfo(
      dto.perpMarketIndex,
      I80F48.from(dto.maintBaseAssetWeight),
      I80F48.from(dto.initBaseAssetWeight),
      I80F48.from(dto.maintBaseLiabWeight),
      I80F48.from(dto.initBaseLiabWeight),
      I80F48.from(dto.maintOverallAssetWeight),
      I80F48.from(dto.initOverallAssetWeight),
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
      perpMarket.maintBaseAssetWeight,
      perpMarket.initBaseAssetWeight,
      perpMarket.maintBaseLiabWeight,
      perpMarket.initBaseLiabWeight,
      perpMarket.maintOverallAssetWeight,
      perpMarket.initOverallAssetWeight,
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
    );
  }

  healthContribution(healthType: HealthType | undefined): I80F48 {
    const contrib = this.unweightedHealthContribution(healthType);
    if (contrib.gt(ZERO_I80F48())) {
      const assetWeight =
        healthType == HealthType.init || healthType == HealthType.liquidationEnd
          ? this.initOverallAssetWeight
          : this.maintOverallAssetWeight;
      return assetWeight.mul(contrib);
    }
    return contrib;
  }

  unweightedHealthContribution(healthType: HealthType | undefined): I80F48 {
    function orderExecutionCase(
      pi: PerpInfo,
      ordersBaseLots: BN,
      orderPrice: I80F48,
    ): I80F48 {
      const netBaseNative = I80F48.fromU64(
        pi.baseLots.add(ordersBaseLots).mul(pi.baseLotSize),
      );

      let weight, basePrice;
      if (
        healthType == HealthType.init ||
        healthType == HealthType.liquidationEnd
      ) {
        if (netBaseNative.isNeg()) {
          weight = pi.initBaseLiabWeight;
        } else {
          weight = pi.initBaseAssetWeight;
        }
      }
      // healthType == HealthType.maint
      else {
        if (netBaseNative.isNeg()) {
          weight = pi.maintBaseLiabWeight;
        } else {
          weight = pi.maintBaseAssetWeight;
        }
      }

      if (netBaseNative.isNeg()) {
        basePrice = pi.prices.liab(healthType);
      } else {
        basePrice = pi.prices.asset(healthType);
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
      perpMarket.maintBaseAssetWeight,
      perpMarket.initBaseAssetWeight,
      perpMarket.maintBaseLiabWeight,
      perpMarket.initBaseLiabWeight,
      perpMarket.maintOverallAssetWeight,
      perpMarket.initOverallAssetWeight,
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
    );
  }

  toString(): string {
    return `  perpMarketIndex: ${this.perpMarketIndex}, base: ${
      this.baseLots
    }, quote: ${this.quote}, oraclePrice: ${
      this.prices.oracle
    }, uncapped health contribution ${this.unweightedHealthContribution(
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
  initScaledAssetWeight: I80F48Dto;
  maintLiabWeight: I80F48Dto;
  initLiabWeight: I80F48Dto;
  initScaledLiabWeight: I80F48Dto;
  prices: { oracle: I80F48Dto; stable: I80F48Dto };
  balanceNative: I80F48Dto;

  constructor(
    tokenIndex: number,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    initScaledAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    initScaledLiabWeight: I80F48Dto,
    prices: { oracle: I80F48Dto; stable: I80F48Dto },
    balanceNative: I80F48Dto,
  ) {
    this.tokenIndex = tokenIndex;
    this.maintAssetWeight = maintAssetWeight;
    this.initAssetWeight = initAssetWeight;
    this.initScaledAssetWeight = initScaledAssetWeight;
    this.maintLiabWeight = maintLiabWeight;
    this.initLiabWeight = initLiabWeight;
    this.initScaledLiabWeight = initScaledLiabWeight;
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
  maintBaseAssetWeight: I80F48Dto;
  initBaseAssetWeight: I80F48Dto;
  maintBaseLiabWeight: I80F48Dto;
  initBaseLiabWeight: I80F48Dto;
  maintOverallAssetWeight: I80F48Dto;
  initOverallAssetWeight: I80F48Dto;
  public baseLotSize: BN;
  public baseLots: BN;
  public bidsBaseLots: BN;
  public asksBaseLots: BN;
  quote: I80F48Dto;
  prices: { oracle: I80F48Dto; stable: I80F48Dto };
  hasOpenOrders: boolean;
}
