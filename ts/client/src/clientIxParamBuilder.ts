import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { InterestRateParams, OracleConfigParams } from './types';

export interface TokenRegisterParams {
  oracleConfig: OracleConfigParams;
  groupInsuranceFund: boolean;
  interestRateParams: InterestRateParams;
  loanFeeRate: number;
  loanOriginationFeeRate: number;
  maintAssetWeight: number;
  initAssetWeight: number;
  maintLiabWeight: number;
  initLiabWeight: number;
  liquidationFee: number;
  stablePriceDelayIntervalSeconds: number;
  stablePriceDelayGrowthLimit: number;
  stablePriceGrowthLimit: number;
  minVaultToDepositsRatio: number;
  netBorrowLimitPerWindowQuote: number;
  netBorrowLimitWindowSizeTs: number;
  borrowWeightScaleStartQuote: number;
  depositWeightScaleStartQuote: number;
  reduceOnly: number;
  tokenConditionalSwapTakerFeeRate: number;
  tokenConditionalSwapMakerFeeRate: number;
  flashLoanSwapFeeRate: number;
  interestCurveScaling: number;
  interestTargetUtilization: number;
  depositLimit: BN;
  zeroUtilRate: number;
  platformLiquidationFee: number;
  disableAssetLiquidation: boolean;
  collateralFeePerDay: number;
}

export const DefaultTokenRegisterParams: TokenRegisterParams = {
  oracleConfig: {
    confFilter: 0,
    maxStalenessSlots: null,
  },
  groupInsuranceFund: false,
  interestRateParams: {
    util0: 0.5,
    rate0: 0.018,
    util1: 0.8,
    rate1: 0.05,
    maxRate: 0.5,
    adjustmentFactor: 0.004,
  },
  loanFeeRate: 0.0005,
  loanOriginationFeeRate: 0.005,
  maintAssetWeight: 0,
  initAssetWeight: 0,
  maintLiabWeight: 1.4,
  initLiabWeight: 1.8,
  liquidationFee: 0.2,
  stablePriceDelayIntervalSeconds: 60 * 60,
  stablePriceDelayGrowthLimit: 0.06,
  stablePriceGrowthLimit: 0.0003,
  minVaultToDepositsRatio: 0.2,
  netBorrowLimitPerWindowQuote: 5_000_000_000,
  netBorrowLimitWindowSizeTs: 86_400,
  borrowWeightScaleStartQuote: 5_000_000_000,
  depositWeightScaleStartQuote: 5_000_000_000,
  reduceOnly: 0,
  tokenConditionalSwapTakerFeeRate: 0.0005,
  tokenConditionalSwapMakerFeeRate: 0.0005,
  flashLoanSwapFeeRate: 0.0005,
  interestCurveScaling: 4.0,
  interestTargetUtilization: 0.5,
  depositLimit: new BN(0),
  zeroUtilRate: 0.0,
  platformLiquidationFee: 0.0,
  disableAssetLiquidation: false,
  collateralFeePerDay: 0.0,
};

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
  reduceOnly: number | null;
  name: string | null;
  forceClose: boolean | null;
  tokenConditionalSwapTakerFeeRate: number | null;
  tokenConditionalSwapMakerFeeRate: number | null;
  flashLoanSwapFeeRate: number | null;
  interestCurveScaling: number | null;
  interestTargetUtilization: number | null;
  maintWeightShiftStart: BN | null;
  maintWeightShiftEnd: BN | null;
  maintWeightShiftAssetTarget: number | null;
  maintWeightShiftLiabTarget: number | null;
  maintWeightShiftAbort: boolean | null;
  fallbackOracle: PublicKey | null;
  depositLimit: BN | null;
  zeroUtilRate: number | null;
  platformLiquidationFee: number | null;
  disableAssetLiquidation: boolean | null;
  collateralFeePerDay: number | null;
  forceWithdraw: boolean | null;
}

export const NullTokenEditParams: TokenEditParams = {
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
  name: null,
  forceClose: null,
  tokenConditionalSwapTakerFeeRate: null,
  tokenConditionalSwapMakerFeeRate: null,
  flashLoanSwapFeeRate: null,
  interestCurveScaling: null,
  interestTargetUtilization: null,
  maintWeightShiftStart: null,
  maintWeightShiftEnd: null,
  maintWeightShiftAssetTarget: null,
  maintWeightShiftLiabTarget: null,
  maintWeightShiftAbort: null,
  fallbackOracle: null,
  depositLimit: null,
  zeroUtilRate: null,
  platformLiquidationFee: null,
  disableAssetLiquidation: null,
  collateralFeePerDay: null,
  forceWithdraw: null,
};

export interface PerpEditParams {
  oracle: PublicKey | null;
  oracleConfig: OracleConfigParams | null;
  baseDecimals: number | null;
  maintBaseAssetWeight: number | null;
  initBaseAssetWeight: number | null;
  maintBaseLiabWeight: number | null;
  initBaseLiabWeight: number | null;
  maintOverallAssetWeight: number | null;
  initOverallAssetWeight: number | null;
  baseLiquidationFee: number | null;
  makerFee: number | null;
  takerFee: number | null;
  feePenalty: number | null;
  minFunding: number | null;
  maxFunding: number | null;
  impactQuantity: number | null;
  groupInsuranceFund: boolean | null;
  settleFeeFlat: number | null;
  settleFeeAmountThreshold: number | null;
  settleFeeFractionLowHealth: number | null;
  stablePriceDelayIntervalSeconds: number | null;
  stablePriceDelayGrowthLimit: number | null;
  stablePriceGrowthLimit: number | null;
  settlePnlLimitFactor: number | null;
  settlePnlLimitWindowSize: number | null;
  reduceOnly: boolean | null;
  resetStablePrice: boolean | null;
  positivePnlLiquidationFee: number | null;
  name: string | null;
  forceClose: boolean | null;
  platformLiquidationFee: number | null;
}

export const NullPerpEditParams: PerpEditParams = {
  oracle: null,
  oracleConfig: null,
  baseDecimals: null,
  maintBaseAssetWeight: null,
  initBaseAssetWeight: null,
  maintBaseLiabWeight: null,
  initBaseLiabWeight: null,
  maintOverallAssetWeight: null,
  initOverallAssetWeight: null,
  baseLiquidationFee: null,
  makerFee: null,
  takerFee: null,
  feePenalty: null,
  minFunding: null,
  maxFunding: null,
  impactQuantity: null,
  groupInsuranceFund: null,
  settleFeeFlat: null,
  settleFeeAmountThreshold: null,
  settleFeeFractionLowHealth: null,
  stablePriceDelayIntervalSeconds: null,
  stablePriceDelayGrowthLimit: null,
  stablePriceGrowthLimit: null,
  settlePnlLimitFactor: null,
  settlePnlLimitWindowSize: null,
  reduceOnly: null,
  resetStablePrice: null,
  positivePnlLiquidationFee: null,
  name: null,
  forceClose: null,
  platformLiquidationFee: null,
};

// Use with TrueIxGateParams and buildIxGate
export interface IxGateParams {
  AccountClose: boolean;
  AccountCreate: boolean;
  AccountEdit: boolean;
  AccountExpand: boolean;
  AccountToggleFreeze: boolean;
  AltExtend: boolean;
  AltSet: boolean;
  FlashLoan: boolean;
  GroupClose: boolean;
  GroupCreate: boolean;
  GroupToggleHalt: boolean;
  HealthRegion: boolean;
  PerpCancelAllOrders: boolean;
  PerpCancelAllOrdersBySide: boolean;
  PerpCancelOrder: boolean;
  PerpCancelOrderByClientOrderId: boolean;
  PerpCloseMarket: boolean;
  PerpConsumeEvents: boolean;
  PerpCreateMarket: boolean;
  PerpDeactivatePosition: boolean;
  PerpEditMarket: boolean;
  PerpLiqBaseOrPositivePnl: boolean;
  PerpLiqForceCancelOrders: boolean;
  PerpLiqNegativePnlOrBankruptcy: boolean;
  PerpPlaceOrder: boolean;
  PerpSettleFees: boolean;
  PerpSettlePnl: boolean;
  PerpUpdateFunding: boolean;
  Serum3CancelAllOrders: boolean;
  Serum3CancelOrder: boolean;
  Serum3CloseOpenOrders: boolean;
  Serum3CreateOpenOrders: boolean;
  Serum3DeregisterMarket: boolean;
  Serum3EditMarket: boolean;
  Serum3LiqForceCancelOrders: boolean;
  Serum3PlaceOrder: boolean;
  Serum3RegisterMarket: boolean;
  Serum3SettleFunds: boolean;
  StubOracleClose: boolean;
  StubOracleCreate: boolean;
  StubOracleSet: boolean;
  TokenAddBank: boolean;
  TokenDeposit: boolean;
  TokenDeregister: boolean;
  TokenEdit: boolean;
  TokenLiqBankruptcy: boolean;
  TokenLiqWithToken: boolean;
  TokenRegister: boolean;
  TokenRegisterTrustless: boolean;
  TokenUpdateIndexAndRate: boolean;
  TokenWithdraw: boolean;
  AccountBuybackFeesWithMngo: boolean;
  TokenForceCloseBorrowsWithToken: boolean;
  PerpForceClosePosition: boolean;
  GroupWithdrawInsuranceFund: boolean;
  TokenConditionalSwapCreate: boolean;
  TokenConditionalSwapTrigger: boolean;
  TokenConditionalSwapCancel: boolean;
  OpenbookV2CancelOrder: boolean;
  OpenbookV2CloseOpenOrders: boolean;
  OpenbookV2CreateOpenOrders: boolean;
  OpenbookV2DeregisterMarket: boolean;
  OpenbookV2EditMarket: boolean;
  OpenbookV2LiqForceCancelOrders: boolean;
  OpenbookV2PlaceOrder: boolean;
  OpenbookV2PlaceTakeOrder: boolean;
  OpenbookV2RegisterMarket: boolean;
  OpenbookV2SettleFunds: boolean;
  AdminTokenWithdrawFees: boolean;
  AdminPerpWithdrawFees: boolean;
  AccountSizeMigration: boolean;
  TokenConditionalSwapStart: boolean;
  TokenConditionalSwapCreatePremiumAuction: boolean;
  TokenConditionalSwapCreateLinearAuction: boolean;
  Serum3PlaceOrderV2: boolean;
  TokenForceWithdraw: boolean;
}

// Default with all ixs enabled, use with buildIxGate
export const TrueIxGateParams: IxGateParams = {
  AccountClose: true,
  AccountCreate: true,
  AccountEdit: true,
  AccountExpand: true,
  AccountToggleFreeze: true,
  AltExtend: true,
  AltSet: true,
  FlashLoan: true,
  GroupClose: true,
  GroupCreate: true,
  GroupToggleHalt: true,
  HealthRegion: true,
  PerpCancelAllOrders: true,
  PerpCancelAllOrdersBySide: true,
  PerpCancelOrder: true,
  PerpCancelOrderByClientOrderId: true,
  PerpCloseMarket: true,
  PerpConsumeEvents: true,
  PerpCreateMarket: true,
  PerpDeactivatePosition: true,
  PerpEditMarket: true,
  PerpLiqBaseOrPositivePnl: true,
  PerpLiqForceCancelOrders: true,
  PerpLiqNegativePnlOrBankruptcy: true,
  PerpPlaceOrder: true,
  PerpSettleFees: true,
  PerpSettlePnl: true,
  PerpUpdateFunding: true,
  Serum3CancelAllOrders: true,
  Serum3CancelOrder: true,
  Serum3CloseOpenOrders: true,
  Serum3CreateOpenOrders: true,
  Serum3DeregisterMarket: true,
  Serum3EditMarket: true,
  Serum3LiqForceCancelOrders: true,
  Serum3PlaceOrder: true,
  Serum3RegisterMarket: true,
  Serum3SettleFunds: true,
  StubOracleClose: true,
  StubOracleCreate: true,
  StubOracleSet: true,
  TokenAddBank: true,
  TokenDeposit: true,
  TokenDeregister: true,
  TokenEdit: true,
  TokenLiqBankruptcy: true,
  TokenLiqWithToken: true,
  TokenRegister: true,
  TokenRegisterTrustless: true,
  TokenUpdateIndexAndRate: true,
  TokenWithdraw: true,
  AccountBuybackFeesWithMngo: true,
  TokenForceCloseBorrowsWithToken: true,
  PerpForceClosePosition: true,
  GroupWithdrawInsuranceFund: true,
  TokenConditionalSwapCreate: true,
  TokenConditionalSwapTrigger: true,
  TokenConditionalSwapCancel: true,
  OpenbookV2CancelOrder: true,
  OpenbookV2CloseOpenOrders: true,
  OpenbookV2CreateOpenOrders: true,
  OpenbookV2DeregisterMarket: true,
  OpenbookV2EditMarket: true,
  OpenbookV2LiqForceCancelOrders: true,
  OpenbookV2PlaceOrder: true,
  OpenbookV2PlaceTakeOrder: true,
  OpenbookV2RegisterMarket: true,
  OpenbookV2SettleFunds: true,
  AdminTokenWithdrawFees: true,
  AdminPerpWithdrawFees: true,
  AccountSizeMigration: true,
  TokenConditionalSwapStart: true,
  TokenConditionalSwapCreatePremiumAuction: true,
  TokenConditionalSwapCreateLinearAuction: true,
  Serum3PlaceOrderV2: true,
  TokenForceWithdraw: true,
};

// build ix gate e.g. buildIxGate(Builder(TrueIxGateParams).TokenDeposit(false).build()).toNumber(),
export function buildIxGate(p: IxGateParams): BN {
  const ixGate = new BN(0);

  function toggleIx(
    ixGate: BN,
    p: IxGateParams,
    propName: string,
    index: number,
  ): void {
    if (p[propName] === undefined) {
      throw new Error(`Unknown property ${propName}`);
    }
    ixGate.ior(p[propName] ? new BN(0) : new BN(1).ushln(index));
  }
  toggleIx(ixGate, p, 'AccountClose', 0);
  toggleIx(ixGate, p, 'AccountCreate', 1);
  toggleIx(ixGate, p, 'AccountEdit', 2);
  toggleIx(ixGate, p, 'AccountExpand', 3);
  toggleIx(ixGate, p, 'AccountToggleFreeze', 4);
  toggleIx(ixGate, p, 'AltExtend', 5);
  toggleIx(ixGate, p, 'AltSet', 6);
  toggleIx(ixGate, p, 'FlashLoan', 7);
  toggleIx(ixGate, p, 'GroupClose', 8);
  toggleIx(ixGate, p, 'GroupCreate', 9);
  toggleIx(ixGate, p, 'HealthRegion', 10);
  toggleIx(ixGate, p, 'PerpCancelAllOrders', 11);
  toggleIx(ixGate, p, 'PerpCancelAllOrdersBySide', 12);
  toggleIx(ixGate, p, 'PerpCancelOrder', 13);
  toggleIx(ixGate, p, 'PerpCancelOrderByClientOrderId', 14);
  toggleIx(ixGate, p, 'PerpCloseMarket', 15);
  toggleIx(ixGate, p, 'PerpConsumeEvents', 16);
  toggleIx(ixGate, p, 'PerpCreateMarket', 17);
  toggleIx(ixGate, p, 'PerpDeactivatePosition', 18);
  toggleIx(ixGate, p, 'PerpLiqBaseOrPositivePnl', 19);
  toggleIx(ixGate, p, 'PerpLiqForceCancelOrders', 20);
  toggleIx(ixGate, p, 'PerpLiqNegativePnlOrBankruptcy', 21);
  toggleIx(ixGate, p, 'PerpPlaceOrder', 22);
  toggleIx(ixGate, p, 'PerpSettleFees', 23);
  toggleIx(ixGate, p, 'PerpSettlePnl', 24);
  toggleIx(ixGate, p, 'PerpUpdateFunding', 25);
  toggleIx(ixGate, p, 'Serum3CancelAllOrders', 26);
  toggleIx(ixGate, p, 'Serum3CancelOrder', 27);
  toggleIx(ixGate, p, 'Serum3CloseOpenOrders', 28);
  toggleIx(ixGate, p, 'Serum3CreateOpenOrders', 29);
  toggleIx(ixGate, p, 'Serum3DeregisterMarket', 30);
  toggleIx(ixGate, p, 'Serum3EditMarket', 31);
  toggleIx(ixGate, p, 'Serum3LiqForceCancelOrders', 32);
  toggleIx(ixGate, p, 'Serum3PlaceOrder', 33);
  toggleIx(ixGate, p, 'Serum3RegisterMarket', 34);
  toggleIx(ixGate, p, 'Serum3SettleFunds', 35);
  toggleIx(ixGate, p, 'StubOracleClose', 36);
  toggleIx(ixGate, p, 'StubOracleCreate', 37);
  toggleIx(ixGate, p, 'StubOracleSet', 38);
  toggleIx(ixGate, p, 'TokenAddBank', 39);
  toggleIx(ixGate, p, 'TokenDeposit', 40);
  toggleIx(ixGate, p, 'TokenDeregister', 41);
  toggleIx(ixGate, p, 'TokenLiqBankruptcy', 42);
  toggleIx(ixGate, p, 'TokenLiqWithToken', 43);
  toggleIx(ixGate, p, 'TokenRegister', 44);
  toggleIx(ixGate, p, 'TokenRegisterTrustless', 45);
  toggleIx(ixGate, p, 'TokenUpdateIndexAndRate', 46);
  toggleIx(ixGate, p, 'TokenWithdraw', 47);
  toggleIx(ixGate, p, 'AccountBuybackFeesWithMngo', 48);
  toggleIx(ixGate, p, 'TokenForceCloseBorrowsWithToken', 49);
  toggleIx(ixGate, p, 'PerpForceClosePosition', 50);
  toggleIx(ixGate, p, 'GroupWithdrawInsuranceFund', 51);
  toggleIx(ixGate, p, 'TokenConditionalSwapCreate', 52);
  toggleIx(ixGate, p, 'TokenConditionalSwapTrigger', 53);
  toggleIx(ixGate, p, 'TokenConditionalSwapCancel', 54);
  toggleIx(ixGate, p, 'OpenbookV2CancelOrder', 55);
  toggleIx(ixGate, p, 'OpenbookV2CloseOpenOrders', 56);
  toggleIx(ixGate, p, 'OpenbookV2CreateOpenOrders', 57);
  toggleIx(ixGate, p, 'OpenbookV2DeregisterMarket', 58);
  toggleIx(ixGate, p, 'OpenbookV2EditMarket', 59);
  toggleIx(ixGate, p, 'OpenbookV2LiqForceCancelOrders', 60);
  toggleIx(ixGate, p, 'OpenbookV2PlaceOrder', 61);
  toggleIx(ixGate, p, 'OpenbookV2PlaceTakeOrder', 62);
  toggleIx(ixGate, p, 'OpenbookV2RegisterMarket', 63);
  toggleIx(ixGate, p, 'OpenbookV2SettleFunds', 63);
  toggleIx(ixGate, p, 'AdminTokenWithdrawFees', 65);
  toggleIx(ixGate, p, 'AdminPerpWithdrawFees', 66);
  toggleIx(ixGate, p, 'AccountSizeMigration', 67);
  toggleIx(ixGate, p, 'TokenConditionalSwapStart', 68);
  toggleIx(ixGate, p, 'TokenConditionalSwapCreatePremiumAuction', 69);
  toggleIx(ixGate, p, 'TokenConditionalSwapCreateLinearAuction', 70);
  toggleIx(ixGate, p, 'Serum3PlaceOrderV2', 71);
  toggleIx(ixGate, p, 'TokenForceWithdraw', 72);

  return ixGate;
}
