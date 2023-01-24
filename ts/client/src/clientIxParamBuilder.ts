import { BN } from '@project-serum/anchor';
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
};

export interface PerpEditParams {
  oracle: PublicKey | null;
  oracleConfig: OracleConfigParams | null;
  baseDecimals: number | null;
  maintBaseAssetWeight: number | null;
  initBaseAssetWeight: number | null;
  maintBaseLiabWeight: number | null;
  initBaseLiabWeight: number | null;
  maintPnlAssetWeight: number | null;
  initPnlAssetWeight: number | null;
  liquidationFee: number | null;
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
}

export const NullPerpEditParams: PerpEditParams = {
  oracle: null,
  oracleConfig: null,
  baseDecimals: null,
  maintBaseAssetWeight: null,
  initBaseAssetWeight: null,
  maintBaseLiabWeight: null,
  initBaseLiabWeight: null,
  maintPnlAssetWeight: null,
  initPnlAssetWeight: null,
  liquidationFee: null,
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
  PerpLiqBasePosition: boolean;
  PerpLiqForceCancelOrders: boolean;
  PerpLiqQuoteAndBankruptcy: boolean;
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
  PerpLiqBasePosition: true,
  PerpLiqForceCancelOrders: true,
  PerpLiqQuoteAndBankruptcy: true,
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
};

// build ix gate e.g. buildIxGate(Builder(TrueIxGateParams).TokenDeposit(false).build()).toNumber(),
export function buildIxGate(p: IxGateParams): BN {
  const ixGate = new BN(0);
  ixGate.ior(p.AccountClose ? new BN(0) : new BN(1).ushln(0));
  ixGate.ior(p.AccountCreate ? new BN(0) : new BN(1).ushln(1));
  ixGate.ior(p.AccountEdit ? new BN(0) : new BN(1).ushln(2));
  ixGate.ior(p.AccountExpand ? new BN(0) : new BN(1).ushln(3));
  ixGate.ior(p.AccountToggleFreeze ? new BN(0) : new BN(1).ushln(4));
  ixGate.ior(p.AltExtend ? new BN(0) : new BN(1).ushln(5));
  ixGate.ior(p.AltSet ? new BN(0) : new BN(1).ushln(6));
  ixGate.ior(p.FlashLoan ? new BN(0) : new BN(1).ushln(7));
  ixGate.ior(p.GroupClose ? new BN(0) : new BN(1).ushln(8));
  ixGate.ior(p.GroupCreate ? new BN(0) : new BN(1).ushln(9));

  ixGate.ior(p.PerpCancelAllOrders ? new BN(0) : new BN(1).ushln(10));
  ixGate.ior(p.PerpCancelAllOrdersBySide ? new BN(0) : new BN(1).ushln(11));
  ixGate.ior(p.PerpCancelOrder ? new BN(0) : new BN(1).ushln(12));
  ixGate.ior(
    p.PerpCancelOrderByClientOrderId ? new BN(0) : new BN(1).ushln(13),
  );
  ixGate.ior(p.PerpCloseMarket ? new BN(0) : new BN(1).ushln(14));
  ixGate.ior(p.PerpConsumeEvents ? new BN(0) : new BN(1).ushln(15));
  ixGate.ior(p.PerpCreateMarket ? new BN(0) : new BN(1).ushln(16));
  ixGate.ior(p.PerpDeactivatePosition ? new BN(0) : new BN(1).ushln(17));
  ixGate.ior(p.PerpLiqBasePosition ? new BN(0) : new BN(1).ushln(18));
  ixGate.ior(p.PerpLiqForceCancelOrders ? new BN(0) : new BN(1).ushln(19));

  ixGate.ior(p.PerpLiqQuoteAndBankruptcy ? new BN(0) : new BN(1).ushln(20));
  ixGate.ior(p.PerpPlaceOrder ? new BN(0) : new BN(1).ushln(21));
  ixGate.ior(p.PerpSettleFees ? new BN(0) : new BN(1).ushln(22));
  ixGate.ior(p.PerpSettlePnl ? new BN(0) : new BN(1).ushln(23));
  ixGate.ior(p.PerpUpdateFunding ? new BN(0) : new BN(1).ushln(24));
  ixGate.ior(p.Serum3CancelAllOrders ? new BN(0) : new BN(1).ushln(25));
  ixGate.ior(p.Serum3CancelOrder ? new BN(0) : new BN(1).ushln(26));
  ixGate.ior(p.Serum3CloseOpenOrders ? new BN(0) : new BN(1).ushln(27));
  ixGate.ior(p.Serum3CreateOpenOrders ? new BN(0) : new BN(1).ushln(28));
  ixGate.ior(p.Serum3DeregisterMarket ? new BN(0) : new BN(1).ushln(29));

  ixGate.ior(p.Serum3EditMarket ? new BN(0) : new BN(1).ushln(30));
  ixGate.ior(p.Serum3LiqForceCancelOrders ? new BN(0) : new BN(1).ushln(31));
  ixGate.ior(p.Serum3PlaceOrder ? new BN(0) : new BN(1).ushln(32));
  ixGate.ior(p.Serum3RegisterMarket ? new BN(0) : new BN(1).ushln(33));
  ixGate.ior(p.Serum3SettleFunds ? new BN(0) : new BN(1).ushln(34));
  ixGate.ior(p.StubOracleClose ? new BN(0) : new BN(1).ushln(35));
  ixGate.ior(p.StubOracleCreate ? new BN(0) : new BN(1).ushln(36));
  ixGate.ior(p.StubOracleSet ? new BN(0) : new BN(1).ushln(37));
  ixGate.ior(p.TokenAddBank ? new BN(0) : new BN(1).ushln(38));
  ixGate.ior(p.TokenDeposit ? new BN(0) : new BN(1).ushln(39));

  ixGate.ior(p.TokenDeregister ? new BN(0) : new BN(1).ushln(40));
  ixGate.ior(p.TokenLiqBankruptcy ? new BN(0) : new BN(1).ushln(41));
  ixGate.ior(p.TokenLiqWithToken ? new BN(0) : new BN(1).ushln(42));
  ixGate.ior(p.TokenRegister ? new BN(0) : new BN(1).ushln(43));
  ixGate.ior(p.TokenRegisterTrustless ? new BN(0) : new BN(1).ushln(44));
  ixGate.ior(p.TokenUpdateIndexAndRate ? new BN(0) : new BN(1).ushln(45));
  ixGate.ior(p.TokenWithdraw ? new BN(0) : new BN(1).ushln(46));

  return ixGate;
}
