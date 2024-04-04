import { BN } from '@coral-xyz/anchor';

export type Modify<T, R> = Omit<T, keyof R> & R;

export class FlashLoanWithdraw {
  static index: number;
  static amount: BN;
}

export type FlashLoanType =
  | { unknown: Record<string, never> }
  | { swap: Record<string, never> }
  | { swapWithoutFee: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace FlashLoanType {
  export const unknown = { unknown: {} };
  export const swap = { swap: {} };
  export const swapWithoutFee = { swapWithoutFee: {} };
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
