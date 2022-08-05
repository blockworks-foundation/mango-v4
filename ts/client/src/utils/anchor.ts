import {
  simulateTransaction,
  SuccessfulTxSimulationResponse,
} from '@frahman5/anchor/dist/cjs/utils/rpc';
import {
  Signer,
  PublicKey,
  Transaction,
  Commitment,
  SimulatedTransactionResponse,
} from '@solana/web3.js';

class SimulateError extends Error {
  constructor(
    readonly simulationResponse: SimulatedTransactionResponse,
    message?: string,
  ) {
    super(message);
  }
}

export async function simulate(
  tx: Transaction,
  signers?: Signer[],
  commitment?: Commitment,
  includeAccounts?: boolean | PublicKey[],
): Promise<SuccessfulTxSimulationResponse> {
  tx.feePayer = this.wallet.publicKey;
  tx.recentBlockhash = (
    await this.connection.getLatestBlockhash(
      commitment ?? this.connection.commitment,
    )
  ).blockhash;

  const result = await simulateTransaction(this.connection, tx);

  if (result.value.err) {
    throw new SimulateError(result.value);
  }

  return result.value;
}
