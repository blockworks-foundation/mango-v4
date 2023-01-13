import { PublicKey } from '@solana/web3.js';
import { InterestRateParams, OracleConfigParams } from './types';

export interface TokenEditParams {
  oracle: PublicKey | null;
  oracleConfig: OracleConfigParams | null;
  groupInsuranceFund: boolean | null;
  interestRateParams: InterestRateParams | null;
  loanFeeRate: number | null;
  loanOriginationFeeRate: number | null;
  maintAssetWeight: number | null;
  initAssetWeight: number | null;
  maintLiabWeight: number | null;
  initLiabWeight: number | null;
  liquidationFee: number | null;
  stablePriceDelayIntervalSeconds: number | null;
  stablePriceDelayGrowthLimit: number | null;
  stablePriceGrowthLimit: number | null;
  minVaultToDepositsRatio: number | null;
  netBorrowLimitPerWindowQuote: number | null;
  netBorrowLimitWindowSizeTs: number | null;
  borrowWeightScaleStartQuote: number | null;
  depositWeightScaleStartQuote: number | null;
  resetStablePrice: boolean | null;
  resetNetBorrowLimit: boolean | null;
  reduceOnly: boolean | null;
}

export const defaultTokenEditParams: TokenEditParams = {
  oracle: null,
  oracleConfig: null,
  groupInsuranceFund: null,
  interestRateParams: null,
  loanFeeRate: null,
  loanOriginationFeeRate: null,
  maintAssetWeight: null,
  initAssetWeight: null,
  maintLiabWeight: null,
  initLiabWeight: null,
  liquidationFee: null,
  stablePriceDelayIntervalSeconds: null,
  stablePriceDelayGrowthLimit: null,
  stablePriceGrowthLimit: null,
  minVaultToDepositsRatio: null,
  netBorrowLimitPerWindowQuote: null,
  netBorrowLimitWindowSizeTs: null,
  borrowWeightScaleStartQuote: null,
  depositWeightScaleStartQuote: null,
  resetStablePrice: null,
  resetNetBorrowLimit: null,
  reduceOnly: null,
};

export interface PerpEditParams {
  oracle: PublicKey | null;
  oracleConfig: OracleConfigParams | null;
  baseDecimals: number | null;
  maintAssetWeight: number | null;
  initAssetWeight: number | null;
  maintLiabWeight: number | null;
  initLiabWeight: number | null;
  liquidationFee: number | null;
  makerFee: number | null;
  takerFee: number | null;
  feePenalty: number | null;
  minFunding: number | null;
  maxFunding: number | null;
  impactQuantity: number | null;
  groupInsuranceFund: boolean | null;
  trustedMarket: boolean | null;
  settleFeeFlat: number | null;
  settleFeeAmountThreshold: number | null;
  settleFeeFractionLowHealth: number | null;
  stablePriceDelayIntervalSeconds: number | null;
  stablePriceDelayGrowthLimit: number | null;
  stablePriceGrowthLimit: number | null;
  settlePnlLimitFactor: number | null;
  settlePnlLimitWindowSize: number | null;
  reduceOnly: boolean | null;
}

export const defaultPerpEditParams: PerpEditParams = {
  oracle: null,
  oracleConfig: null,
  baseDecimals: null,
  maintAssetWeight: null,
  initAssetWeight: null,
  maintLiabWeight: null,
  initLiabWeight: null,
  liquidationFee: null,
  makerFee: null,
  takerFee: null,
  feePenalty: null,
  minFunding: null,
  maxFunding: null,
  impactQuantity: null,
  groupInsuranceFund: null,
  trustedMarket: null,
  settleFeeFlat: null,
  settleFeeAmountThreshold: null,
  settleFeeFractionLowHealth: null,
  stablePriceDelayIntervalSeconds: null,
  stablePriceDelayGrowthLimit: null,
  stablePriceGrowthLimit: null,
  settlePnlLimitFactor: null,
  settlePnlLimitWindowSize: null,
  reduceOnly: null,
};
