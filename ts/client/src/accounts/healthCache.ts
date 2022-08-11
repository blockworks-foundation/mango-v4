import _ from 'lodash';
import { Group } from './group';
import {
  HUNDRED_I80F48,
  I80F48,
  I80F48Dto,
  MAX_I80F48,
  ZERO_I80F48,
} from './I80F48';
import { HealthType } from './mangoAccount';

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
  tokenInfos: TokenInfo[];
  serum3Infos: Serum3Info[];
  perpInfos: PerpInfo[];

  constructor(dto: HealthCacheDto) {
    this.tokenInfos = dto.tokenInfos.map((dto) => new TokenInfo(dto));
    this.serum3Infos = dto.serum3Infos.map((dto) => new Serum3Info(dto));
    this.perpInfos = dto.perpInfos.map((dto) => new PerpInfo(dto));
  }

  public health(healthType: HealthType): I80F48 {
    let health = ZERO_I80F48;
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      health = health.add(contrib);
    }
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
      );
      health = health.add(contrib);
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      health = health.add(contrib);
    }
    return health;
  }

  public assets(healthType: HealthType): I80F48 {
    let assets = ZERO_I80F48;
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets = assets.add(contrib);
      }
    }
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
      );
      if (contrib.isPos()) {
        assets = assets.add(contrib);
      }
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets = assets.add(contrib);
      }
    }
    return assets;
  }

  public liabs(healthType: HealthType): I80F48 {
    let liabs = ZERO_I80F48;
    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      if (contrib.isNeg()) {
        liabs = liabs.sub(contrib);
      }
    }
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
      );
      if (contrib.isNeg()) {
        liabs = liabs.sub(contrib);
      }
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      if (contrib.isNeg()) {
        liabs = liabs.sub(contrib);
      }
    }
    return liabs;
  }

  public healthRatio(healthType: HealthType): I80F48 {
    let assets = ZERO_I80F48;
    let liabs = ZERO_I80F48;

    for (const tokenInfo of this.tokenInfos) {
      const contrib = tokenInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets = assets.add(contrib);
      } else {
        liabs = liabs.sub(contrib);
      }
    }
    for (const serum3Info of this.serum3Infos) {
      const contrib = serum3Info.healthContribution(
        healthType,
        this.tokenInfos,
      );
      if (contrib.isPos()) {
        assets = assets.add(contrib);
      } else {
        liabs = liabs.sub(contrib);
      }
    }
    for (const perpInfo of this.perpInfos) {
      const contrib = perpInfo.healthContribution(healthType);
      if (contrib.isPos()) {
        assets = assets.add(contrib);
      } else {
        liabs = liabs.sub(contrib);
      }
    }

    if (liabs.isPos()) {
      return HUNDRED_I80F48.add(assets.sub(liabs).div(liabs));
    } else {
      return MAX_I80F48;
    }
  }

  findTokenInfoIndex(tokenIndex: number): number {
    return this.tokenInfos.findIndex(
      (tokenInfo) => tokenInfo.tokenIndex == tokenIndex,
    );
  }

  getMaxSourceForTokenSwap(
    group: Group,
    sourceTokenName: string,
    targetTokenName: string,
    minRatio: I80F48,
  ): I80F48 {
    const sourceTokenIndex = group.banksMap.get(sourceTokenName).tokenIndex;
    const targetTokenIndex = group.banksMap.get(targetTokenName).tokenIndex;

    // The health_ratio is a nonlinear based on swap amount.
    // For large swap amounts the slope is guaranteed to be negative, but small amounts
    // can have positive slope (e.g. using source deposits to pay back target borrows).
    //
    // That means:
    // - even if the initial ratio is < minRatio it can be useful to swap to *increase* health
    // - be careful about finding the minRatio point: the function isn't convex

    const initialRatio = this.healthRatio(HealthType.init);
    if (initialRatio.lte(ZERO_I80F48)) {
      return ZERO_I80F48;
    }

    const sourceIndex = this.findTokenInfoIndex(sourceTokenIndex);
    const targetIndex = this.findTokenInfoIndex(targetTokenIndex);

    const source = this.tokenInfos[sourceIndex];
    const target = this.tokenInfos[targetIndex];

    // There are two key slope changes: Assume source.balance > 0 and target.balance < 0. Then
    // initially health ratio goes up. When one of balances flips sign, the health ratio slope
    // may be positive or negative for a bit, until both balances have flipped and the slope is
    // negative.
    // The maximum will be at one of these points (ignoring serum3 effects).
    const originalHealthCache: HealthCache = _.cloneDeep(this);
    function cacheAfterSwap(amount: I80F48) {
      const adjustedCache: HealthCache = _.cloneDeep(originalHealthCache);
      adjustedCache.tokenInfos[sourceIndex].balance =
        adjustedCache.tokenInfos[sourceIndex].balance.sub(amount);
      adjustedCache.tokenInfos[targetIndex].balance =
        adjustedCache.tokenInfos[targetIndex].balance.add(amount);
      return adjustedCache;
    }

    function healthRatioAfterSwap(amount: I80F48): I80F48 {
      return cacheAfterSwap(amount).healthRatio(HealthType.init);
    }

    const point0Amount = source.balance
      .min(target.balance.neg())
      .max(ZERO_I80F48);
    const point1Amount = source.balance
      .max(target.balance.neg())
      .max(ZERO_I80F48);
    const point0Ratio = healthRatioAfterSwap(point0Amount);
    const cache = cacheAfterSwap(point1Amount);
    const point1Ratio = cache.healthRatio(HealthType.init);
    const point1Health = cache.health(HealthType.init);

    function binaryApproximationSearch(
      left: I80F48,
      leftRatio: I80F48,
      right: I80F48,
      rightRatio: I80F48,
      targetRatio: I80F48,
    ) {
      const maxIterations = 20;
      const targetError = I80F48.fromString('0.000001'); // ONE_I80F48;

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

      for (const key of Array(maxIterations).fill(0).keys()) {
        const newAmount = left.add(right).mul(I80F48.fromString('0.5'));
        const newAmountRatio = healthRatioAfterSwap(newAmount);
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
      throw new Error(
        `Unable to get targetRatio within ${maxIterations} iterations`,
      );
    }

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
        amount = ZERO_I80F48;
      }
    } else if (point1Ratio.gte(minRatio)) {
      // If point1Ratio is still bigger than minRatio, the target amount must be >point1Amount
      // search to the right of point1Amount: but how far?
      // At point1, source.balance < 0 and target.balance > 0, so use a simple estimation for
      // zero health: health - source_liab_weight * a + target_asset_weight * a = 0.
      if (point1Health.lte(ZERO_I80F48)) {
        return ZERO_I80F48;
      }
      const zeroHealthAmount = point1Amount.add(
        point1Health.div(source.initLiabWeight.sub(target.initAssetWeight)),
      );
      const zeroHealthRatio = healthRatioAfterSwap(zeroHealthAmount);
      amount = binaryApproximationSearch(
        point1Amount,
        point1Ratio,
        zeroHealthAmount,
        zeroHealthRatio,
        minRatio,
      );
    } else if (point0Ratio.gte(minRatio)) {
      // Must be between point0Amount and point1Amount.
      amount = binaryApproximationSearch(
        point0Amount,
        point0Ratio,
        point1Amount,
        point1Ratio,
        minRatio,
      );
    } else {
      throw new Error(
        `internal error: assert that init ratio ${initialRatio.toNumber()} <= point0 ratio ${point0Ratio.toNumber()}`,
      );
    }

    return amount.div(source.oraclePrice);
  }
}

export class TokenInfo {
  constructor(dto: TokenInfoDto) {
    this.tokenIndex = dto.tokenIndex;
    this.maintAssetWeight = I80F48.from(dto.maintAssetWeight);
    this.initAssetWeight = I80F48.from(dto.initAssetWeight);
    this.maintLiabWeight = I80F48.from(dto.maintLiabWeight);
    this.initLiabWeight = I80F48.from(dto.initLiabWeight);
    this.oraclePrice = I80F48.from(dto.oraclePrice);
    this.balance = I80F48.from(dto.balance);
    this.serum3MaxReserved = I80F48.from(dto.serum3MaxReserved);
  }

  tokenIndex: number;
  maintAssetWeight: I80F48;
  initAssetWeight: I80F48;
  maintLiabWeight: I80F48;
  initLiabWeight: I80F48;
  oraclePrice: I80F48; // native/native
  // in health-reference-token native units
  balance: I80F48;
  // in health-reference-token native units
  serum3MaxReserved: I80F48;

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
}

export class Serum3Info {
  constructor(dto: Serum3InfoDto) {
    this.reserved = I80F48.from(dto.reserved);
    this.baseIndex = dto.baseIndex;
    this.quoteIndex = dto.quoteIndex;
  }

  reserved: I80F48;
  baseIndex: number;
  quoteIndex: number;

  healthContribution(healthType: HealthType, tokenInfos: TokenInfo[]): I80F48 {
    const baseInfo = tokenInfos[this.baseIndex];
    const quoteInfo = tokenInfos[this.quoteIndex];
    const reserved = this.reserved;

    if (reserved.isZero()) {
      return ZERO_I80F48;
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
        liabPart = ZERO_I80F48;
      } else if (maxBalance.isNeg()) {
        assetPart = ZERO_I80F48;
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
}

export class PerpInfo {
  constructor(dto: PerpInfoDto) {
    this.maintAssetWeight = I80F48.from(dto.maintAssetWeight);
    this.initAssetWeight = I80F48.from(dto.initAssetWeight);
    this.maintLiabWeight = I80F48.from(dto.maintLiabWeight);
    this.initLiabWeight = I80F48.from(dto.initLiabWeight);
    this.base = I80F48.from(dto.base);
    this.quote = I80F48.from(dto.quote);
  }
  maintAssetWeight: I80F48;
  initAssetWeight: I80F48;
  maintLiabWeight: I80F48;
  initLiabWeight: I80F48;
  // in health-reference-token native units, needs scaling by asset/liab
  base: I80F48;
  // in health-reference-token native units, no asset/liab factor needed
  quote: I80F48;

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
    return this.quote.add(weight.mul(this.base)).min(ZERO_I80F48);
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
}

export class Serum3InfoDto {
  reserved: I80F48Dto;
  baseIndex: number;
  quoteIndex: number;
}

export class PerpInfoDto {
  maintAssetWeight: I80F48Dto;
  initAssetWeight: I80F48Dto;
  maintLiabWeight: I80F48Dto;
  initLiabWeight: I80F48Dto;
  // in health-reference-token native units, needs scaling by asset/liab
  base: I80F48Dto;
  // in health-reference-token native units, no asset/liab factor needed
  quote: I80F48Dto;
}
