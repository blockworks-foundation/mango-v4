import { PublicKey } from '@solana/web3.js';
import _ from 'lodash';
import { Bank } from './bank';
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
    this.tokenInfos = dto.tokenInfos.map((dto) => TokenInfo.fromDto(dto));
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
      return HUNDRED_I80F48.mul(assets.sub(liabs).div(liabs));
    } else {
      return MAX_I80F48;
    }
  }

  findTokenInfoIndex(tokenIndex: number): number {
    return this.tokenInfos.findIndex(
      (tokenInfo) => tokenInfo.tokenIndex == tokenIndex,
    );
  }

  getOrCreateTokenInfoIndex(bank: Bank): number {
    const index = this.findTokenInfoIndex(bank.tokenIndex);
    if (index == -1) {
      this.tokenInfos.push(TokenInfo.emptyFromBank(bank));
    }
    return this.findTokenInfoIndex(bank.tokenIndex);
  }

  private static logHealthCache(debug: string, healthCache: HealthCache) {
    console.log(debug);
    for (const token of healthCache.tokenInfos) {
      console.log(`${token.toString()}`);
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
      adjustedCache.tokenInfos[changeIndex].balance = adjustedCache.tokenInfos[
        changeIndex
      ].balance.add(change.nativeTokenAmount.mul(bank.price));
    }
    // HealthCache.logHealthCache('afterChange', adjustedCache);
    return adjustedCache.healthRatio(healthType);
  }

  getMaxSourceForTokenSwap(
    group: Group,
    sourceMintPk: PublicKey,
    targetMintPk: PublicKey,
    minRatio: I80F48,
  ): I80F48 {
    const sourceBank: Bank = group.getFirstBankByMint(sourceMintPk);
    const targetBank: Bank = group.getFirstBankByMint(targetMintPk);

    if (sourceMintPk.equals(targetMintPk)) {
      return ZERO_I80F48;
    }

    if (!sourceBank.price || sourceBank.price.lte(ZERO_I80F48)) {
      return ZERO_I80F48;
    }

    if (
      sourceBank.initLiabWeight
        .sub(targetBank.initAssetWeight)
        .abs()
        .lte(ZERO_I80F48)
    ) {
      return ZERO_I80F48;
    }

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
      adjustedCache.tokenInfos[sourceIndex].balance =
        adjustedCache.tokenInfos[sourceIndex].balance.sub(amount);
      adjustedCache.tokenInfos[targetIndex].balance =
        adjustedCache.tokenInfos[targetIndex].balance.add(amount);
      // HealthCache.logHealthCache('afterSwap', adjustedCache);
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
    const cache0 = cacheAfterSwap(point0Amount);
    const point0Ratio = cache0.healthRatio(HealthType.init);
    const point0Health = cache0.health(HealthType.init);
    const cache1 = cacheAfterSwap(point1Amount);
    const point1Ratio = cache1.healthRatio(HealthType.init);
    const point1Health = cache1.health(HealthType.init);

    function binaryApproximationSearch(
      left: I80F48,
      leftRatio: I80F48,
      right: I80F48,
      rightRatio: I80F48,
      targetRatio: I80F48,
    ) {
      const maxIterations = 20;
      // TODO: make relative to health ratio decimals? Might be over engineering
      const targetError = I80F48.fromString('0.001');

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
        newAmount = left.add(right).mul(I80F48.fromString('0.5'));
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
      console.error(
        `Unable to get targetRatio within ${maxIterations} iterations`,
      );
      return newAmount;
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
      // console.log(`point1Amount ${point1Amount}`);
      // console.log(`point1Health ${point1Health}`);
      // console.log(`point1Ratio ${point1Ratio}`);
      // console.log(`point0Amount ${point0Amount}`);
      // console.log(`point0Health ${point0Health}`);
      // console.log(`point0Ratio ${point0Ratio}`);
      // console.log(`zeroHealthAmount ${zeroHealthAmount}`);
      const zeroHealthRatio = healthRatioAfterSwap(zeroHealthAmount);
      // console.log(`zeroHealthRatio ${zeroHealthRatio}`);
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

    return amount
      .div(source.oraclePrice)
      .div(
        ONE_I80F48.add(
          group.getFirstBankByMint(sourceMintPk).loanOriginationFeeRate,
        ),
      );
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

  static emptyFromBank(bank: Bank): TokenInfo {
    return new TokenInfo(
      bank.tokenIndex,
      bank.maintAssetWeight,
      bank.initAssetWeight,
      bank.maintLiabWeight,
      bank.initLiabWeight,
      bank.price,
      ZERO_I80F48,
      ZERO_I80F48,
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
    return `  tokenIndex: ${this.tokenIndex}, balance: ${this.balance}`;
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
