import { BN } from '@coral-xyz/anchor';
import { OpenOrders } from '@project-serum/serum';
import { PublicKey } from '@solana/web3.js';
import { I80F48, MAX_I80F48, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import {
  deepClone,
  toNativeI80F48ForQuote,
  toUiDecimals,
  toUiDecimalsForQuote,
} from '../utils';
import { Bank, BankForHealth, TokenIndex } from './bank';
import { Group } from './group';

import {
  HealthType,
  MangoAccount,
  PerpPosition,
  Serum3Orders,
} from './mangoAccount';
import { PerpMarket, PerpMarketIndex, PerpOrder, PerpOrderSide } from './perp';
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

function spotAmountTakenForHealthZero(
  health: I80F48,
  startingSpot: I80F48,
  assetWeightedPrice: I80F48,
  liabWeightedPrice: I80F48,
): I80F48 {
  if (health.lte(ZERO_I80F48())) {
    return ZERO_I80F48();
  }

  let takenSpot = ZERO_I80F48();
  if (startingSpot.gt(ZERO_I80F48())) {
    if (assetWeightedPrice.gt(ZERO_I80F48())) {
      const assetMax = health.div(assetWeightedPrice);
      if (assetMax.lte(startingSpot)) {
        return assetMax;
      }
    }
    takenSpot = startingSpot;
    health.isub(startingSpot.mul(assetWeightedPrice));
  }
  if (health.gt(ZERO_I80F48())) {
    if (liabWeightedPrice.lte(ZERO_I80F48())) {
      throw new Error('LiabWeightedPrice must be greater than 0!');
    }
    takenSpot.iadd(health.div(liabWeightedPrice));
  }
  return takenSpot;
}

function spotAmountGivenForHealthZero(
  health: I80F48,
  startingSpot: I80F48,
  assetWeightedPrice: I80F48,
  liabWeightedPrice: I80F48,
): I80F48 {
  return spotAmountTakenForHealthZero(
    health.neg(),
    startingSpot.neg(),
    liabWeightedPrice,
    assetWeightedPrice,
  );
}

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

    // if no usdc position is found, insert it nonetheless, this is required for simulating
    // 1st max perp trade
    if (
      !tokenInfos.find(
        (ti) =>
          ti.tokenIndex == group.getFirstBankForPerpSettlement().tokenIndex,
      )
    ) {
      tokenInfos.push(
        TokenInfo.fromBank(
          group.getFirstBankForPerpSettlement(),
          ZERO_I80F48(),
        ),
      );
    }
    // Fill the TokenInfo balance with free funds in serum3 oo accounts, and fill
    // the serum3MaxReserved with their reserved funds. Also build Serum3Infos.
    const serum3Infos = mangoAccount.serum3Active().map((serum3) => {
      const oo = mangoAccount.getSerum3OoAccount(serum3.marketIndex);

      // find the TokenInfos for the market's base and quote tokens
      const baseInfoIndex = tokenInfos.findIndex(
        (tokenInfo) => tokenInfo.tokenIndex === serum3.baseTokenIndex,
      );
      const baseInfo = tokenInfos[baseInfoIndex];
      if (!baseInfo) {
        throw new Error(
          `BaseInfo not found for market with marketIndex ${serum3.marketIndex}!`,
        );
      }
      const quoteInfoIndex = tokenInfos.findIndex(
        (tokenInfo) => tokenInfo.tokenIndex === serum3.quoteTokenIndex,
      );
      const quoteInfo = tokenInfos[quoteInfoIndex];
      if (!quoteInfo) {
        throw new Error(
          `QuoteInfo not found for market with marketIndex ${serum3.marketIndex}!`,
        );
      }

      return Serum3Info.fromOoModifyingTokenInfos(
        serum3,
        baseInfoIndex,
        baseInfo,
        quoteInfoIndex,
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

  computeSerum3Reservations(healthType: HealthType | undefined): {
    tokenMaxReserved: TokenMaxReserved[];
    serum3Reserved: Serum3Reserved[];
  } {
    // For each token, compute the sum of serum-reserved amounts over all markets.
    const tokenMaxReserved = new Array(this.tokenInfos.length)
      .fill(null)
      .map((ignored) => new TokenMaxReserved(ZERO_I80F48()));

    // For each serum market, compute what happened if reserved_base was converted to quote
    // or reserved_quote was converted to base.
    const serum3Reserved: Serum3Reserved[] = [];

    for (const info of this.serum3Infos) {
      const quote = this.tokenInfos[info.quoteInfoIndex];
      const base = this.tokenInfos[info.baseInfoIndex];

      const reservedBase = info.reservedBase;
      const reservedQuote = info.reservedQuote;

      const quoteAsset = quote.prices.asset(healthType);
      const baseLiab = base.prices.liab(healthType);
      const reservedQuoteAsBaseOracle = reservedQuote.mul(
        quoteAsset.div(baseLiab),
      );
      let allReservedAsBase;
      if (!info.reservedQuoteAsBaseHighestBid.eq(ZERO_I80F48())) {
        allReservedAsBase = reservedBase.add(
          reservedQuoteAsBaseOracle.min(info.reservedQuoteAsBaseHighestBid),
        );
      } else {
        allReservedAsBase = reservedBase.add(reservedQuoteAsBaseOracle);
      }

      const baseAsset = base.prices.asset(healthType);
      const quoteLiab = quote.prices.liab(healthType);
      const reservedBaseAsQuoteOracle = reservedBase.mul(
        baseAsset.div(quoteLiab),
      );
      let allReservedAsQuote;
      if (!info.reservedBaseAsQuoteLowestAsk.eq(ZERO_I80F48())) {
        allReservedAsQuote = reservedQuote.add(
          reservedBaseAsQuoteOracle.min(info.reservedBaseAsQuoteLowestAsk),
        );
      } else {
        allReservedAsQuote = reservedQuote.add(reservedBaseAsQuoteOracle);
      }

      const baseMaxReserved = tokenMaxReserved[info.baseInfoIndex];
      baseMaxReserved.maxSerumReserved.iadd(allReservedAsBase);
      const quoteMaxReserved = tokenMaxReserved[info.quoteInfoIndex];
      quoteMaxReserved.maxSerumReserved.iadd(allReservedAsQuote);

      serum3Reserved.push(
        new Serum3Reserved(allReservedAsBase, allReservedAsQuote),
      );
    }

    return {
      tokenMaxReserved: tokenMaxReserved,
      serum3Reserved: serum3Reserved,
    };
  }

  effectiveTokenBalances(healthType: HealthType | undefined): TokenBalance[] {
    return this.effectiveTokenBalancesInternal(healthType, false);
  }

  effectiveTokenBalancesInternal(
    healthType: HealthType | undefined,
    ignoreNegativePerp: boolean,
  ): TokenBalance[] {
    const tokenBalances = new Array(this.tokenInfos.length)
      .fill(null)
      .map((ignored) => new TokenBalance(ZERO_I80F48()));

    for (const perpInfo of this.perpInfos) {
      const settleTokenIndex = this.findTokenInfoIndex(
        perpInfo.settleTokenIndex,
      );
      const perpSettleToken = tokenBalances[settleTokenIndex];
      const healthUnsettled = perpInfo.healthUnsettledPnl(healthType);
      if (!ignoreNegativePerp || healthUnsettled.gt(ZERO_I80F48())) {
        perpSettleToken.spotAndPerp.iadd(healthUnsettled);
      }
    }

    for (const index of this.tokenInfos.keys()) {
      const tokenInfo = this.tokenInfos[index];
      const tokenBalance = tokenBalances[index];
      tokenBalance.spotAndPerp.iadd(tokenInfo.balanceSpot);
    }

    return tokenBalances;
  }

  effectiveTokenBalancesInternalDisplay(
    group: Group,
    healthType: HealthType | undefined,
    ignoreNegativePerp: boolean,
  ): TokenBalanceDisplay[] {
    const tokenBalances = new Array(this.tokenInfos.length)
      .fill(null)
      .map((ignored) => new TokenBalanceDisplay(ZERO_I80F48(), 0, []));

    for (const perpInfo of this.perpInfos) {
      const settleTokenIndex = this.findTokenInfoIndex(
        perpInfo.settleTokenIndex,
      );
      const perpSettleToken = tokenBalances[settleTokenIndex];
      const healthUnsettled = perpInfo.healthUnsettledPnl(healthType);
      perpSettleToken.perpMarketContributions.push({
        market: group.getPerpMarketByMarketIndex(
          perpInfo.perpMarketIndex as PerpMarketIndex,
        ).name,
        contributionUi: toUiDecimals(
          healthUnsettled,
          group.getMintDecimalsByTokenIndex(perpInfo.settleTokenIndex),
        ),
      });
      if (!ignoreNegativePerp || healthUnsettled.gt(ZERO_I80F48())) {
        perpSettleToken.spotAndPerp.iadd(healthUnsettled);
      }
    }

    for (const index of this.tokenInfos.keys()) {
      const tokenInfo = this.tokenInfos[index];
      const tokenBalance = tokenBalances[index];
      tokenBalance.spotAndPerp.iadd(tokenInfo.balanceSpot);
      tokenBalance.spotUi += toUiDecimals(
        tokenInfo.balanceSpot,
        group.getMintDecimalsByTokenIndex(tokenInfo.tokenIndex),
      );
    }

    return tokenBalances;
  }

  healthSum(healthType: HealthType, tokenBalances: TokenBalance[]): I80F48 {
    const health = ZERO_I80F48();
    for (const index of this.tokenInfos.keys()) {
      const tokenInfo = this.tokenInfos[index];
      const tokenBalance = tokenBalances[index];
      const contrib = tokenInfo.healthContribution(
        healthType,
        tokenBalance.spotAndPerp,
      );
      // console.log(` - ti ${contrib}`);
      health.iadd(contrib);
    }
    const res = this.computeSerum3Reservations(healthType);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
        tokenBalances,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      // console.log(` - si ${contrib}`);
      health.iadd(contrib);
    }
    return health;
  }

  healthContributionPerAssetUi(
    group: Group,
    healthType: HealthType,
  ): {
    asset: string;
    contribution: number;
    contributionDetails:
      | {
          spotUi: number;
          perpMarketContributions: { market: string; contributionUi: number }[];
        }
      | undefined;
  }[] {
    const tokenBalancesDisplay: TokenBalanceDisplay[] =
      this.effectiveTokenBalancesInternalDisplay(group, healthType, false);

    const ret = new Array<{
      asset: string;
      contribution: number;
      contributionDetails:
        | {
            spotUi: number;
            perpMarketContributions: {
              market: string;
              contributionUi: number;
            }[];
          }
        | undefined;
    }>();
    for (const index of this.tokenInfos.keys()) {
      const tokenInfo = this.tokenInfos[index];
      const tokenBalance = tokenBalancesDisplay[index];
      const contrib = tokenInfo.healthContribution(
        healthType,
        tokenBalance.spotAndPerp,
      );
      ret.push({
        asset: group.getFirstBankByTokenIndex(tokenInfo.tokenIndex).name,
        contribution: toUiDecimalsForQuote(contrib),
        contributionDetails: {
          spotUi: tokenBalance.spotUi,
          perpMarketContributions: tokenBalance.perpMarketContributions,
        },
      });
    }
    const res = this.computeSerum3Reservations(healthType);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
        tokenBalancesDisplay,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      ret.push({
        asset: group.getSerum3MarketByMarketIndex(serum3Info.marketIndex).name,
        contribution: toUiDecimalsForQuote(contrib),
        contributionDetails: undefined,
      });
    }

    return ret;
  }

  public health(healthType: HealthType): I80F48 {
    const tokenBalances = this.effectiveTokenBalancesInternal(
      healthType,
      false,
    );
    return this.healthSum(healthType, tokenBalances);
  }

  public perpMaxSettle(settleTokenIndex: TokenIndex): I80F48 {
    const healthType = HealthType.maint;
    const tokenBalances = this.effectiveTokenBalancesInternal(healthType, true);
    const perpSettleHealth = this.healthSum(healthType, tokenBalances);
    const tokenInfoIndex = this.findTokenInfoIndex(settleTokenIndex);
    const tokenInfo = this.tokenInfos[tokenInfoIndex];
    return spotAmountTakenForHealthZero(
      perpSettleHealth,
      tokenBalances[tokenInfoIndex].spotAndPerp,
      tokenInfo.assetWeightedPrice(healthType),
      tokenInfo.liabWeightedPrice(healthType),
    );
  }

  healthAssetsAndLiabsStableAssets(healthType: HealthType): {
    assets: I80F48;
    liabs: I80F48;
  } {
    return this.healthAssetsAndLiabs(healthType, true);
  }

  healthAssetsAndLiabsStableLiabs(healthType: HealthType): {
    assets: I80F48;
    liabs: I80F48;
  } {
    return this.healthAssetsAndLiabs(healthType, false);
  }

  public healthAssetsAndLiabs(
    healthType: HealthType | undefined,
    stableAssets: boolean,
  ): { assets: I80F48; liabs: I80F48 } {
    const totalAssets = ZERO_I80F48();
    const totalLiabs = ZERO_I80F48();
    for (const tokenInfo of this.tokenInfos) {
      const assetBalance = ZERO_I80F48();
      const liabBalance = ZERO_I80F48();

      if (tokenInfo.balanceSpot.isPos()) {
        assetBalance.iadd(tokenInfo.balanceSpot);
      } else {
        liabBalance.isub(tokenInfo.balanceSpot);
      }

      for (const perpInfo of this.perpInfos) {
        if (perpInfo.settleTokenIndex != tokenInfo.tokenIndex) {
          continue;
        }
        const healthUnsettled = perpInfo.healthUnsettledPnl(healthType);
        if (healthUnsettled.isPos()) {
          assetBalance.iadd(healthUnsettled);
        } else {
          liabBalance.isub(healthUnsettled);
        }
      }

      if (stableAssets) {
        const assetWeightedPrice = tokenInfo.assetWeightedPrice(healthType);
        const assets = assetBalance.mul(assetWeightedPrice);
        totalAssets.iadd(assets);
        if (assetBalance.gte(liabBalance)) {
          totalLiabs.iadd(liabBalance.mul(assetWeightedPrice));
        } else {
          const liabWeightedPrice = tokenInfo.liabWeightedPrice(healthType);
          totalLiabs.iadd(
            assets.add(liabBalance.sub(assetBalance).mul(liabWeightedPrice)),
          );
        }
      } else {
        const liabWeightedPrice = tokenInfo.liabWeightedPrice(healthType);
        const liabs = liabBalance.mul(liabWeightedPrice);
        totalLiabs.iadd(liabs);
        if (assetBalance.gte(liabBalance)) {
          const assetWeightedPrice = tokenInfo.assetWeightedPrice(healthType);
          totalAssets.iadd(
            liabs.add(assetBalance.sub(liabBalance).mul(assetWeightedPrice)),
          );
        } else {
          totalAssets.iadd(assetBalance.mul(liabWeightedPrice));
        }
      }
    }

    const tokenBalances = this.effectiveTokenBalances(healthType);
    const res = this.computeSerum3Reservations(healthType);
    for (const [index, serum3Info] of this.serum3Infos.entries()) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
        tokenBalances,
        res.tokenMaxReserved,
        res.serum3Reserved[index],
      );
      if (contrib.isPos()) {
        totalAssets.iadd(contrib);
      } else {
        totalLiabs.iadd(contrib);
      }
    }

    return { assets: totalAssets, liabs: totalLiabs };
  }

  public healthRatio(healthType: HealthType): I80F48 {
    const res = this.healthAssetsAndLiabsStableLiabs(healthType);
    const hundred = I80F48.fromNumber(100);
    // console.log(`assets ${res.assets}`);
    // console.log(`liabs ${res.liabs}`);
    if (res.liabs.gt(I80F48.fromNumber(0.001))) {
      return hundred.mul(res.assets.sub(res.liabs)).div(res.liabs);
    }
    return MAX_I80F48();
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
    const adjustedCache: HealthCache = deepClone<HealthCache>(this);
    // HealthCache.logHealthCache('beforeChange', adjustedCache);
    for (const change of nativeTokenChanges) {
      const bank: Bank = group.getFirstBankByMint(change.mintPk);
      const changeIndex = adjustedCache.getOrCreateTokenInfoIndex(bank);
      // TODO: this will no longer work as easily because of the health weight changes
      adjustedCache.tokenInfos[changeIndex].balanceSpot.iadd(
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
    baseEntry.balanceSpot.iadd(freeBaseChange);
    quoteEntry.balanceSpot.iadd(freeQuoteChange);

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
    const adjustedCache: HealthCache = deepClone<HealthCache>(this);
    const quoteIndex = adjustedCache.getOrCreateTokenInfoIndex(quoteBank);

    // Move token balance to reserved funds in open orders,
    // essentially simulating a place order

    // Reduce token balance for quote
    adjustedCache.tokenInfos[quoteIndex].balanceSpot.isub(bidNativeQuoteAmount);

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
    const adjustedCache: HealthCache = deepClone<HealthCache>(this);
    const baseIndex = adjustedCache.getOrCreateTokenInfoIndex(baseBank);

    // Move token balance to reserved funds in open orders,
    // essentially simulating a place order

    // Reduce token balance for base
    adjustedCache.tokenInfos[baseIndex].balanceSpot.isub(askNativeBaseAmount);

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
    const clonedHealthCache: HealthCache = deepClone<HealthCache>(this);
    const perpInfoIndex =
      clonedHealthCache.getOrCreatePerpInfoIndex(perpMarket);
    clonedHealthCache.adjustPerpInfo(perpInfoIndex, price, side, baseLots);
    return clonedHealthCache.healthRatio(healthType);
  }

  private static scanRightUntilLessThan(
    start: I80F48,
    target: I80F48,
    fun: (amount: I80F48) => I80F48,
  ): I80F48 {
    const maxIterations = 50;
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

  /// This is not a generic function. It assumes there is a almost-unique maximum between left and right,
  /// in the sense that `fun` might be constant on the maximum value for a while, but there won't be
  /// distinct maximums with non-maximal values between them.
  ///
  /// If the maximum isn't just a single point, it returns the rightmost value.
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
      if (leftValue.gt(midValue)) {
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
        if (leftMidValue.gt(midValue)) {
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

    if (leftValue.gt(midValue)) {
      return [left, leftValue];
    } else if (midValue.gt(rightValue)) {
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
    options: { maxIterations?: number; targetError?: number } = {},
  ): I80F48 {
    const maxIterations = options?.maxIterations || 50;
    const targetError = I80F48.fromNumber(options?.targetError || 0.1);

    const rightValue = fun(right);

    // console.log(
    //   ` - binaryApproximationSearch left ${left.toLocaleString()}, leftValue ${leftValue.toLocaleString()}, right ${right.toLocaleString()}, rightValue ${rightValue.toLocaleString()}, targetValue ${targetValue.toLocaleString()}, minStep ${minStep}`,
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

    const healthCacheClone: HealthCache = deepClone<HealthCache>(this);
    const sourceIndex = healthCacheClone.getOrCreateTokenInfoIndex(sourceBank);
    const targetIndex = healthCacheClone.getOrCreateTokenInfoIndex(targetBank);

    const source = healthCacheClone.tokenInfos[sourceIndex];
    const target = healthCacheClone.tokenInfos[targetIndex];

    const res = healthCacheClone.computeSerum3Reservations(HealthType.init);
    const sourceReserved = res.tokenMaxReserved[sourceIndex].maxSerumReserved;
    const targetReserved = res.tokenMaxReserved[targetIndex].maxSerumReserved;

    const tokenBalances = healthCacheClone.effectiveTokenBalances(
      HealthType.init,
    );
    const sourceBalance = tokenBalances[sourceIndex].spotAndPerp;
    const targetBalance = tokenBalances[targetIndex].spotAndPerp;

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
      const adjustedCache: HealthCache =
        deepClone<HealthCache>(healthCacheClone);
      // adjustedCache.logHealthCache('beforeSwap', adjustedCache);
      // TODO: make a copy of the bank, apply amount, recompute weights,
      // and set the new weights on the tokenInfos
      adjustedCache.tokenInfos[sourceIndex].balanceSpot.isub(amount);
      adjustedCache.tokenInfos[targetIndex].balanceSpot.iadd(amount.mul(price));
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
    const rightmost = sourceBalance
      .abs()
      .add(sourceReserved)
      .max(targetBalance.abs().add(targetReserved).div(price));
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
    if (healthAtMaxValue.eq(ZERO_I80F48())) {
      return amountForMaxValue;
    } else if (healthAtMaxValue.lt(ZERO_I80F48())) {
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
    const healthCacheClone: HealthCache = deepClone<HealthCache>(this);

    const baseIndex = healthCacheClone.getOrCreateTokenInfoIndex(baseBank);
    const quoteIndex = healthCacheClone.getOrCreateTokenInfoIndex(quoteBank);
    const base = healthCacheClone.tokenInfos[baseIndex];
    const quote = healthCacheClone.tokenInfos[quoteIndex];

    const res = healthCacheClone.computeSerum3Reservations(HealthType.init);
    const baseReserved = res.tokenMaxReserved[baseIndex].maxSerumReserved;
    const quoteReserved = res.tokenMaxReserved[quoteIndex].maxSerumReserved;

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
    // amount = M + (init_health + M * (B_init_liab - A_init_asset) + R) / (A_init_liab - B_init_asset);
    // where M = max(A_deposits, B_borrows)
    // and R = reserved serum A amount (because they might offset A borrows)
    // A is what we would be essentially swapping for B
    // So when its an ask, then base->quote,
    // and when its a bid, then quote->bid
    let zeroAmount;
    if (side == Serum3Side.ask) {
      const quoteBorrows = quote.balanceSpot.lt(ZERO_I80F48())
        ? quote.balanceSpot.abs().mul(quote.prices.liab(HealthType.init))
        : ZERO_I80F48();
      const max = base.balanceSpot.mul(base.prices.oracle).max(quoteBorrows);
      zeroAmount = max.add(
        initialHealth
          .add(max.mul(quote.initLiabWeight.sub(base.initScaledAssetWeight)))
          .add(baseReserved.mul(base.prices.liab(HealthType.init)))
          .div(
            base
              .liabWeight(HealthType.init)
              .sub(quote.assetWeight(HealthType.init)),
          ),
      );
      // console.log(` - quoteBorrows ${quoteBorrows.toLocaleString()}`);
      // console.log(` - max ${max.toLocaleString()}`);
    } else {
      const baseBorrows = base.balanceSpot.lt(ZERO_I80F48())
        ? base.balanceSpot.abs().mul(base.prices.liab(HealthType.init))
        : ZERO_I80F48();
      const max = quote.balanceSpot.mul(quote.prices.oracle).max(baseBorrows);
      zeroAmount = max.add(
        initialHealth
          .add(max.mul(base.initLiabWeight.sub(quote.initScaledAssetWeight)))
          .add(quoteReserved.mul(quote.prices.liab(HealthType.init)))
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
      const adjustedCache: HealthCache =
        deepClone<HealthCache>(healthCacheClone);
      // adjustedCache.logHealthCache(` before placing order ${amount}`);
      // TODO: there should also be some issue with oracle vs stable price here;
      // probably better to pass in not the quote amount but the base or quote native amount
      side === Serum3Side.ask
        ? adjustedCache.tokenInfos[baseIndex].balanceSpot.isub(
            amount.div(base.prices.oracle),
          )
        : adjustedCache.tokenInfos[quoteIndex].balanceSpot.isub(
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
    const healthCacheClone: HealthCache = deepClone<HealthCache>(this);

    const initialRatio = this.healthRatio(HealthType.init);
    if (initialRatio.lt(ZERO_I80F48())) {
      return ZERO_I80F48();
    }

    const direction = side == PerpOrderSide.bid ? 1 : -1;

    const perpInfoIndex = healthCacheClone.getOrCreatePerpInfoIndex(perpMarket);
    const perpInfo = healthCacheClone.perpInfos[perpInfoIndex];
    const prices = perpInfo.basePrices;
    const baseLotSize = I80F48.fromI64(perpMarket.baseLotSize);

    const settleInfoIndex = this.findTokenInfoIndex(perpInfo.settleTokenIndex);
    const settleInfo = this.tokenInfos[settleInfoIndex];

    const finalHealthSlope =
      direction == 1
        ? perpInfo.initBaseAssetWeight
            .mul(prices.asset(HealthType.init))
            .sub(price)
        : perpInfo.initBaseLiabWeight
            .neg()
            .mul(prices.liab(HealthType.init))
            .add(price);
    if (finalHealthSlope.gte(ZERO_I80F48())) {
      return MAX_I80F48();
    }
    finalHealthSlope.imul(settleInfo.liabWeightedPrice(HealthType.init));

    function cacheAfterTrade(baseLots: BN): HealthCache {
      const adjustedCache: HealthCache =
        deepClone<HealthCache>(healthCacheClone);
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
      startCache.perpInfos[perpInfoIndex].initOverallAssetWeight = ONE_I80F48();
      const settleInfo = startCache.tokenInfos[settleInfoIndex];
      settleInfo.initAssetWeight = settleInfo.initLiabWeight;
      settleInfo.initScaledAssetWeight = settleInfo.initScaledLiabWeight;
      const startHealth = startCache.health(HealthType.init);
      if (startHealth.lte(ZERO_I80F48())) {
        return ZERO_I80F48();
      }

      const zeroHealthAmount = case1Start
        .sub(
          startHealth.div(
            finalHealthSlope.mul(baseLotSize).mul(I80F48.fromNumber(0.99)),
          ),
        )
        .add(ONE_I80F48());
      const zeroHealthRatio = healthRatioAfterTradeTrunc(zeroHealthAmount);

      // console.log(`case1Start ${case1Start}`);
      // console.log(`case1StartRatio ${case1StartRatio}`);
      // console.log(`zeroHealthAmount ${zeroHealthAmount}`);
      // console.log(`zeroHealthRatio ${zeroHealthRatio}`);
      // console.log(`minRatio ${minRatio}`);

      baseLots = HealthCache.binaryApproximationSearch(
        case1Start,
        case1StartRatio,
        zeroHealthAmount,
        zeroHealthRatio.max(minRatio), // workaround, originally minRatio
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

  public getPerpPositionLiquidationPrice(
    group: Group,
    mangoAccount: MangoAccount,
    perpPosition: PerpPosition,
  ): I80F48 | null {
    const hc = HealthCache.fromMangoAccount(group, mangoAccount);
    const hcClone = deepClone<HealthCache>(hc);
    const perpMarket = group.getPerpMarketByMarketIndex(
      perpPosition.marketIndex,
    );

    function healthAfterPriceChange(newPrice: I80F48): I80F48 {
      const pi: PerpInfo =
        hcClone.perpInfos[hcClone.findPerpInfoIndex(perpPosition.marketIndex)];
      pi.basePrices.oracle = newPrice;
      return hcClone.health(HealthType.maint);
    }

    if (perpPosition.getBasePosition(perpMarket).isPos()) {
      const zero = ZERO_I80F48();
      const healthAtPriceZero = healthAfterPriceChange(zero);
      if (healthAtPriceZero.gt(ZERO_I80F48())) {
        return null;
      }

      return HealthCache.binaryApproximationSearch(
        zero,
        healthAtPriceZero,
        perpMarket.price,
        ZERO_I80F48(),
        perpMarket.priceLotsToNative(new BN(1)),
        healthAfterPriceChange,
      );
    }

    const price1000x = perpMarket.price.mul(I80F48.fromNumber(1000));
    return HealthCache.binaryApproximationSearch(
      perpMarket.price,
      hcClone.health(HealthType.maint),
      price1000x,
      ZERO_I80F48(),
      perpMarket.priceLotsToNative(new BN(1)),
      healthAfterPriceChange,
    );
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
    public balanceSpot: I80F48,
  ) {}

  static fromBank(bank: BankForHealth, nativeBalance?: I80F48): TokenInfo {
    const p = new Prices(
      bank.price,
      I80F48.fromNumber(bank.stablePriceModel.stablePrice),
    );
    // Use the liab price for computing weight scaling, because it's pessimistic and
    // causes the most unfavorable scaling.
    const liabPrice = p.liab(HealthType.init);

    const [maintAssetWeight, maintLiabWeight] = bank.maintWeights();

    return new TokenInfo(
      bank.tokenIndex,
      maintAssetWeight,
      bank.initAssetWeight,
      bank.scaledInitAssetWeight(liabPrice),
      maintLiabWeight,
      bank.initLiabWeight,
      bank.scaledInitLiabWeight(liabPrice),
      p,
      nativeBalance ? nativeBalance : ZERO_I80F48(),
    );
  }

  assetWeight(healthType: HealthType | undefined): I80F48 {
    if (healthType == HealthType.init) {
      return this.initScaledAssetWeight;
    } else if (healthType == HealthType.liquidationEnd) {
      return this.initAssetWeight;
    }
    if (healthType == HealthType.maint) {
      return this.maintAssetWeight;
    }
    return I80F48.fromNumber(1);
  }

  assetWeightedPrice(healthType: HealthType | undefined): I80F48 {
    return this.assetWeight(healthType).mul(this.prices.asset(healthType));
  }

  liabWeight(healthType: HealthType | undefined): I80F48 {
    if (healthType == HealthType.init) {
      return this.initScaledLiabWeight;
    } else if (healthType == HealthType.liquidationEnd) {
      return this.initLiabWeight;
    }
    if (healthType == HealthType.maint) {
      return this.maintLiabWeight;
    }
    return I80F48.fromNumber(1);
  }

  liabWeightedPrice(healthType: HealthType | undefined): I80F48 {
    return this.liabWeight(healthType).mul(this.prices.liab(healthType));
  }

  healthContribution(
    healthType: HealthType | undefined,
    balance: I80F48,
  ): I80F48 {
    if (healthType === undefined) {
      return balance.mul(this.prices.oracle);
    }
    // console.log(`balance ${balance}`);
    return balance.isNeg()
      ? balance.mul(this.liabWeightedPrice(healthType))
      : balance.mul(this.assetWeightedPrice(healthType));
  }

  toString(balance: I80F48): string {
    return `  tokenIndex: ${this.tokenIndex}, balanceNative: ${
      this.balanceSpot
    }, initHealth ${this.healthContribution(HealthType.init, balance)}`;
  }
}

class TokenBalance {
  constructor(public spotAndPerp: I80F48) {}
}

class TokenBalanceDisplay {
  constructor(
    public spotAndPerp: I80F48,
    public spotUi: number,
    public perpMarketContributions: {
      market: string;
      contributionUi: number;
    }[],
  ) {}
}

class TokenMaxReserved {
  constructor(public maxSerumReserved: I80F48) {}
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
    public reservedBaseAsQuoteLowestAsk: I80F48,
    public reservedQuoteAsBaseHighestBid: I80F48,
    public baseInfoIndex: number,
    public quoteInfoIndex: number,
    public marketIndex: MarketIndex,
  ) {}

  static emptyFromSerum3Market(
    serum3Market: Serum3Market,
    baseEntryIndex: number,
    quoteEntryIndex: number,
  ): Serum3Info {
    return new Serum3Info(
      ZERO_I80F48(),
      ZERO_I80F48(),
      ZERO_I80F48(),
      ZERO_I80F48(),
      baseEntryIndex,
      quoteEntryIndex,
      serum3Market.marketIndex,
    );
  }

  static fromOoModifyingTokenInfos(
    serumAccount: Serum3Orders,
    baseInfoIndex: number,
    baseInfo: TokenInfo,
    quoteInfoIndex: number,
    quoteInfo: TokenInfo,
    marketIndex: MarketIndex,
    oo: OpenOrders,
  ): Serum3Info {
    // add the amounts that are freely settleable immediately to token balances
    const baseFree = I80F48.fromI64(oo.baseTokenFree);
    const quoteFree = I80F48.fromI64(oo.quoteTokenFree);
    baseInfo.balanceSpot.iadd(baseFree);
    quoteInfo.balanceSpot.iadd(quoteFree);

    // track the reserved amounts
    const reservedBase = I80F48.fromI64(
      oo.baseTokenTotal.sub(oo.baseTokenFree),
    );
    const reservedQuote = I80F48.fromI64(
      oo.quoteTokenTotal.sub(oo.quoteTokenFree),
    );

    const reservedBaseAsQuoteLowestAsk = reservedBase.mul(
      I80F48.fromNumber(serumAccount.lowestPlacedAsk),
    );
    const reservedQuoteAsBaseHighestBid = reservedQuote.mul(
      I80F48.fromNumber(serumAccount.highestPlacedBidInv),
    );

    return new Serum3Info(
      reservedBase,
      reservedQuote,
      reservedBaseAsQuoteLowestAsk,
      reservedQuoteAsBaseHighestBid,
      baseInfoIndex,
      quoteInfoIndex,
      marketIndex,
    );
  }

  // An undefined HealthType will use an asset and liab weight of 1
  healthContribution(
    healthType: HealthType | undefined,
    tokenInfos: TokenInfo[],
    tokenBalances: TokenBalance[],
    tokenMaxReserved: TokenMaxReserved[],
    marketReserved: Serum3Reserved,
  ): I80F48 {
    if (
      marketReserved.allReservedAsBase.isZero() ||
      marketReserved.allReservedAsQuote.isZero()
    ) {
      return ZERO_I80F48();
    }

    const baseInfo = tokenInfos[this.baseInfoIndex];
    const quoteInfo = tokenInfos[this.quoteInfoIndex];
    const baseMaxReserved = tokenMaxReserved[this.baseInfoIndex];
    const quoteMaxReserved = tokenMaxReserved[this.quoteInfoIndex];

    // How much the health would increase if the reserved balance were applied to the passed
    // token info?
    const computeHealthEffect = function (
      tokenInfo: TokenInfo,
      balance: TokenBalance,
      maxReserved: TokenMaxReserved,
      marketReserved: I80F48,
    ): I80F48 {
      // This balance includes all possible reserved funds from markets that relate to the
      // token, including this market itself: `tokenMaxReserved` is already included in `maxBalance`.
      const maxBalance = balance.spotAndPerp.add(maxReserved.maxSerumReserved);

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
      tokenBalances[this.baseInfoIndex],
      tokenMaxReserved[this.baseInfoIndex],
      marketReserved.allReservedAsBase,
    );
    const healthQuote = computeHealthEffect(
      quoteInfo,
      tokenBalances[this.quoteInfoIndex],
      tokenMaxReserved[this.quoteInfoIndex],
      marketReserved.allReservedAsQuote,
    );

    // console.log(` - healthBase ${healthBase.toLocaleString()}`);
    // console.log(` - healthQuote ${healthQuote.toLocaleString()}`);

    return healthBase.min(healthQuote);
  }

  toString(
    tokenInfos: TokenInfo[],
    tokenBalances: TokenBalance[],
    tokenMaxReserved: TokenMaxReserved[],
    marketReserved: Serum3Reserved,
  ): string {
    return `  marketIndex: ${this.marketIndex}, baseInfoIndex: ${
      this.baseInfoIndex
    }, quoteInfoIndex: ${this.quoteInfoIndex}, reservedBase: ${
      this.reservedBase
    }, reservedQuote: ${
      this.reservedQuote
    }, initHealth ${this.healthContribution(
      HealthType.init,
      tokenInfos,
      tokenBalances,
      tokenMaxReserved,
      marketReserved,
    )}`;
  }
}

export class PerpInfo {
  constructor(
    public perpMarketIndex: number,
    public settleTokenIndex: TokenIndex,
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
    public basePrices: Prices,
    public hasOpenOrders: boolean,
  ) {}

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
      perpMarket.settleTokenIndex,
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

  healthContribution(healthType: HealthType, settleToken: TokenInfo): I80F48 {
    const contrib = this.unweightedHealthUnsettledPnl(healthType);
    return this.weighHealthContributionSettle(
      this.weighHealthContributionOverall(contrib, healthType),
      healthType,
      settleToken,
    );
  }

  healthUnsettledPnl(healthType: HealthType | undefined): I80F48 {
    const contrib = this.unweightedHealthUnsettledPnl(healthType);
    return this.weighHealthContributionOverall(contrib, healthType);
  }

  weighHealthContributionSettle(
    unweighted: I80F48,
    healthType: HealthType,
    settleToken: TokenInfo,
  ): I80F48 {
    if (this.settleTokenIndex !== settleToken.tokenIndex) {
      throw new Error('Settle token index should match!');
    }
    if (unweighted.gt(ZERO_I80F48())) {
      return (
        healthType == HealthType.init
          ? settleToken.initScaledAssetWeight
          : healthType == HealthType.liquidationEnd
          ? settleToken.initAssetWeight
          : settleToken.maintLiabWeight
      )
        .mul(unweighted)
        .mul(settleToken.prices.asset(healthType));
    }
    return (
      healthType == HealthType.init
        ? settleToken.initScaledLiabWeight
        : healthType == HealthType.liquidationEnd
        ? settleToken.initLiabWeight
        : settleToken.maintLiabWeight
    )
      .mul(unweighted)
      .mul(settleToken.prices.liab(healthType));
  }

  weighHealthContributionOverall(
    unweighted: I80F48,
    healthType: HealthType | undefined,
  ): I80F48 {
    if (unweighted.gt(ZERO_I80F48())) {
      return (
        healthType == HealthType.init || healthType == HealthType.liquidationEnd
          ? this.initOverallAssetWeight
          : this.maintOverallAssetWeight
      ).mul(unweighted);
    }
    return unweighted;
  }

  unweightedHealthUnsettledPnl(healthType: HealthType | undefined): I80F48 {
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
        basePrice = pi.basePrices.liab(healthType);
      } else {
        basePrice = pi.basePrices.asset(healthType);
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
      this.basePrices.liab(healthType),
    );
    const asksCase = orderExecutionCase(
      this,
      this.asksBaseLots.neg(),
      this.basePrices.asset(healthType),
    );
    const worstCase = bidsCase.min(asksCase);

    return this.quote.add(worstCase);
  }

  static emptyFromPerpMarket(perpMarket: PerpMarket): PerpInfo {
    return new PerpInfo(
      perpMarket.perpMarketIndex,
      perpMarket.settleTokenIndex,
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
      this.basePrices.oracle
    }, uncapped health contribution ${this.unweightedHealthUnsettledPnl(
      HealthType.init,
    )}`;
  }
}
