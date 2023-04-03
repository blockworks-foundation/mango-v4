import BN from 'bn.js';
import {
  BankForHealth,
  I80F48,
  PerpMarket,
  StablePriceModel,
  TokenIndex,
  ZERO_I80F48,
} from '..';

export function mockBankAndOracle(
  tokenIndex: TokenIndex,
  maintWeight: number,
  initWeight: number,
  price: number,
  stablePrice: number,
): BankForHealth {
  return {
    tokenIndex,
    maintAssetWeight: I80F48.fromNumber(1 - maintWeight),
    initAssetWeight: I80F48.fromNumber(1 - initWeight),
    maintLiabWeight: I80F48.fromNumber(1 + maintWeight),
    initLiabWeight: I80F48.fromNumber(1 + initWeight),
    price: I80F48.fromNumber(price),
    stablePriceModel: { stablePrice: stablePrice } as StablePriceModel,
    scaledInitAssetWeight: () => I80F48.fromNumber(1 - initWeight),
    scaledInitLiabWeight: () => I80F48.fromNumber(1 + initWeight),
  };
}

export function mockPerpMarket(
  perpMarketIndex: number,
  maintBaseWeight: number,
  initBaseWeight: number,
  baseLotSize: number,
  quoteLotSize: number,
  price: number,
): PerpMarket {
  return {
    perpMarketIndex,
    maintBaseAssetWeight: I80F48.fromNumber(1 - maintBaseWeight),
    initBaseAssetWeight: I80F48.fromNumber(1 - initBaseWeight),
    maintBaseLiabWeight: I80F48.fromNumber(1 + maintBaseWeight),
    initBaseLiabWeight: I80F48.fromNumber(1 + initBaseWeight),
    maintOverallAssetWeight: I80F48.fromNumber(1 - 0.02),
    initOverallAssetWeight: I80F48.fromNumber(1 - 0.05),
    price: I80F48.fromNumber(price),
    stablePriceModel: { stablePrice: price } as StablePriceModel,
    baseLotSize: new BN(baseLotSize),
    quoteLotSize: new BN(quoteLotSize),
    longFunding: ZERO_I80F48(),
    shortFunding: ZERO_I80F48(),
  } as unknown as PerpMarket;
}
