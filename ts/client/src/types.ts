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

export type HealthCheckKind =
  | { maint: Record<string, never> }
  | { init: Record<string, never> }
  | { liquidationEnd: Record<string, never> }
  | { maintRatio: Record<string, never> }
  | { initRatio: Record<string, never> }
  | { liquidationEndRatio: Record<string, never> };
// eslint-disable-next-line @typescript-eslint/no-namespace
export namespace HealthCheckKind {
  export const maint = { maint: {} };
  export const init = { init: {} };
  export const liquidationEnd = { liquidationEnd: {} };
  export const maintRatio = { maintRatio: {} };
  export const initRatio = { initRatio: {} };
  export const liquidationEndRatio = { liquidationEndRatio: {} };
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
