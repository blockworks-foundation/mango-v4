import { Connection } from '@solana/web3.js';
import { JUPITER } from './constants';

export enum WellKnownTransactionErrors {
  JupiterSlippageToleranceExceeded,
  Unknown,
}

export function isAJupiterTx(logMessages: string[]): boolean {
  return (
    logMessages.filter(
      (msg) => msg.toLowerCase().indexOf(JUPITER.V3.toBase58()) > -1,
    ).length > 0 ||
    logMessages.filter(
      (msg) => msg.toLowerCase().indexOf(JUPITER.V4.toBase58()) > -1,
    ).length > 0 ||
    logMessages.filter(
      (msg) => msg.toLowerCase().indexOf(JUPITER.V6.toBase58()) > -1,
    ).length > 0
  );
}

export async function parseTxForKnownErrors(
  connection: Connection,
  signature: string,
): Promise<WellKnownTransactionErrors> {
  const tx = await connection.getTransaction(signature, {
    commitment: 'confirmed',
    maxSupportedTransactionVersion: 0,
  });

  if (tx && tx.meta && tx.meta.logMessages) {
    if (
      tx.meta.logMessages.filter(
        (msg) => msg.toLowerCase().indexOf('SlippageToleranceExceeded') > -1,
      ).length > 0 &&
      isAJupiterTx(tx.meta.logMessages)
    ) {
      return WellKnownTransactionErrors.JupiterSlippageToleranceExceeded;
    }
  }

  return WellKnownTransactionErrors.Unknown;
}
