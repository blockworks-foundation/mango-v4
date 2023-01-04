import { BN } from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';

export type Modify<T, R> = Omit<T, keyof R> & R;

export class FlashLoanWithdraw {
  static index: number;
  static amount: BN;
}

export class FlashLoanType {
  static unknown = { unknown: {} };
  static swap = { swap: {} };
}

export class InterestRateParams {
  util0: number;
  rate0: number;
  util1: number;
  rate1: number;
  maxRate: number;
  adjustmentFactor: number;
}

export class OracleConfigParams {
  confFilter: number;
  maxStalenessSlots: number | null;
}

export type AdditionalHealthAccounts = {
  banks: PublicKey[];
  oracles: PublicKey[];
  perps: PublicKey[];
  openOrders: PublicKey[];
};
