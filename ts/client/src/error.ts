import { Connection } from '@solana/web3.js';
import { JUPITER } from './constants';

export enum TransactionErrors {
  // Slippage incurred was higher than user expected
  JupiterSlippageToleranceExceeded,
  Unknown,
}

export function containsJupiterProgram(logMessages: string[]): boolean {
  return (
    logMessages.some((msg) => msg.includes(JUPITER.V3.toBase58())) ||
    logMessages.some((msg) => msg.includes(JUPITER.V4.toBase58())) ||
    logMessages.some((msg) => msg.includes(JUPITER.V6.toBase58()))
  );
}

export async function parseTxForKnownErrors(
  connection: Connection,
  signature: string,
): Promise<TransactionErrors> {
  const tx = await connection.getTransaction(signature, {
    commitment: 'confirmed',
    maxSupportedTransactionVersion: 0,
  });

  if (tx && tx.meta && tx.meta.logMessages) {
    if (
      tx.meta.logMessages.some((msg) =>
        msg.includes('SlippageToleranceExceeded'),
      ) &&
      containsJupiterProgram(tx.meta.logMessages)
    ) {
      return TransactionErrors.JupiterSlippageToleranceExceeded;
    }
  }

  return TransactionErrors.Unknown;
}
