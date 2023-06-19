import { toNative } from '../utils';

const PREMIUM_LISTING_BASE = {
  maxStalenessSlots: 120 as number | null,
  oracleConfFilter: 0.1,
  adjustmentFactor: 0.004,
  util0: 0.5,
  rate0: 0.052,
  util1: 0.8,
  rate1: 0.1446,
  maxRate: 1.4456,
  loanFeeRate: 0.005,
  loanOriginationFeeRate: 0.001,
  maintAssetWeight: 0.9,
  initAssetWeight: 0.8,
  maintLiabWeight: 1.1,
  initLiabWeight: 1.2,
  liquidationFee: 0.05,
  minVaultToDepositsRatio: 0.2,
  netBorrowLimitWindowSizeTs: 24 * 60 * 60,
  netBorrowLimitPerWindowQuote: toNative(50000, 6).toNumber(),
  insuranceFound: true,
  borrowWeightScale: toNative(250000, 6).toNumber(),
  depositWeightScale: toNative(250000, 6).toNumber(),
  name: 'Blue chip',
  key: 'PREMIUM',
};

export type ListingPreset = typeof PREMIUM_LISTING_BASE;

export type LISTING_PRESETS_KEYS =
  | 'PREMIUM'
  | 'MID'
  | 'MEME'
  | 'SHIT'
  | 'UNTRUSTED';

export const LISTING_PRESETS: {
  [key in LISTING_PRESETS_KEYS]: ListingPreset | Record<string, never>;
} = {
  //Price impact on $100,000 swap lower then 1%
  PREMIUM: {
    ...PREMIUM_LISTING_BASE,
    name: 'Blue chip',
    key: 'PREMIUM',
  },
  //Price impact on $20,000 swap lower then 1%
  MID: {
    ...PREMIUM_LISTING_BASE,
    maintAssetWeight: 0.75,
    initAssetWeight: 0.5,
    maintLiabWeight: 1.2,
    initLiabWeight: 1.4,
    liquidationFee: 0.1,
    netBorrowLimitPerWindowQuote: toNative(20000, 6).toNumber(),
    name: 'Midwit',
    key: 'MID',
    borrowWeightScale: toNative(50000, 6).toNumber(),
    depositWeightScale: toNative(50000, 6).toNumber(),
    insuranceFound: false,
  },
  //Price impact on $5,000 swap lower then 1%
  MEME: {
    ...PREMIUM_LISTING_BASE,
    maxStalenessSlots: 800,
    loanOriginationFeeRate: 0.002,
    maintAssetWeight: 0,
    initAssetWeight: 0,
    maintLiabWeight: 1.25,
    initLiabWeight: 1.5,
    liquidationFee: 0.125,
    netBorrowLimitPerWindowQuote: toNative(5000, 6).toNumber(),
    borrowWeightScale: toNative(20000, 6).toNumber(),
    depositWeightScale: toNative(20000, 6).toNumber(),
    insuranceFound: false,
    name: 'Meme Coin',
    key: 'MEME',
  },
  //Price impact on $1,000 swap lower then 1%
  SHIT: {
    ...PREMIUM_LISTING_BASE,
    maxStalenessSlots: 800,
    loanOriginationFeeRate: 0.002,
    maintAssetWeight: 0,
    initAssetWeight: 0,
    maintLiabWeight: 1.4,
    initLiabWeight: 1.8,
    liquidationFee: 0.2,
    netBorrowLimitPerWindowQuote: toNative(1000, 6).toNumber(),
    borrowWeightScale: toNative(5000, 6).toNumber(),
    depositWeightScale: toNative(5000, 6).toNumber(),
    insuranceFound: false,
    name: 'Shit Coin',
    key: 'SHIT',
  },
  //should run untrusted instruction
  UNTRUSTED: {},
};

export type MarketTradingParams = {
  baseLots: number;
  quoteLots: number;
  minOrderValue: number;
  baseLotExponent: number;
  quoteLotExponent: number;
  minOrderSize: number;
  priceIncrement: number;
  priceIncrementRelative: number;
};

// definitions:
// baseLots = 10 ^ baseLotExponent
// quoteLots = 10 ^ quoteLotExponent
// minOrderSize = 10^(baseLotExponent - baseDecimals)
// minOrderValue = basePrice * minOrderSize
// priceIncrement =  10^(quoteLotExponent + baseDecimals - baseLotExponent - quoteDecimals)
// priceIncrementRelative =  priceIncrement * quotePrice / basePrice

// derive: baseLotExponent <= min[ basePrice * minOrderSize > 0.05]
// baseLotExponent = 10
// While (baseLotExponent < 10):
//     minOrderSize =  10^(baseLotExponent - baseDecimals)
//     minOrderValue =  basePrice * minOrderSize
//     if minOrderValue > 0.05:
//         break;

// Derive: quoteLotExponent <= min[ priceIncrement * quotePrice / basePrice > 0.000025 ]
// quoteLotExponent = 0
// While (quoteLotExponent < 10):
//     priceIncrement =  10^(quoteLotExponent + baseDecimals - baseLotExponent - quoteDecimals)
//         priceIncrementRelative =  priceIncrement * quotePrice / basePrice
//     if priceIncrementRelative > 0.000025:
//         break;
export const calculateMarketTradingParams = (
  basePrice: number,
  quotePrice: number,
  baseDecimals: number,
  quoteDecimals: number,
): MarketTradingParams => {
  const MAX_MIN_ORDER_VALUE = 0.05;
  const MIN_PRICE_INCREMENT_RELATIVE = 0.000025;
  const EXPONENT_THRESHOLD = 10;

  let minOrderSize = 0;
  let priceIncrement = 0;
  let baseLotExponent = 0;
  let quoteLotExponent = 0;
  let minOrderValue = 0;
  let priceIncrementRelative = 0;

  // Calculate minimum order size
  do {
    minOrderSize = Math.pow(10, baseLotExponent - baseDecimals);
    minOrderValue = basePrice * minOrderSize;

    if (minOrderValue > MAX_MIN_ORDER_VALUE) {
      break;
    }

    baseLotExponent++;
  } while (baseLotExponent < EXPONENT_THRESHOLD);

  // Calculate price increment
  do {
    priceIncrement = Math.pow(
      10,
      quoteLotExponent + baseDecimals - baseLotExponent - quoteDecimals,
    );
    priceIncrementRelative = (priceIncrement * quotePrice) / basePrice;
    if (priceIncrementRelative > MIN_PRICE_INCREMENT_RELATIVE) {
      break;
    }

    quoteLotExponent++;
  } while (quoteLotExponent < EXPONENT_THRESHOLD);

  //exception override values in that case example eth/btc market
  if (
    quoteLotExponent === 0 &&
    priceIncrementRelative > 0.001 &&
    minOrderSize < 1
  ) {
    baseLotExponent = baseLotExponent + 1;
    minOrderSize = Math.pow(10, baseLotExponent - baseDecimals);
    minOrderValue = basePrice * minOrderSize;
    priceIncrement = Math.pow(
      10,
      quoteLotExponent + baseDecimals - baseLotExponent - quoteDecimals,
    );
    priceIncrementRelative = (priceIncrement * quotePrice) / basePrice;
  }

  return {
    baseLots: Math.pow(10, baseLotExponent),
    quoteLots: Math.pow(10, quoteLotExponent),
    minOrderValue: minOrderValue,
    baseLotExponent: baseLotExponent,
    quoteLotExponent: quoteLotExponent,
    minOrderSize: minOrderSize,
    priceIncrement: priceIncrement,
    priceIncrementRelative: priceIncrementRelative,
  };
};
