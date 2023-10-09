import { AnchorProvider } from '@coral-xyz/anchor';
import NodeWallet from '@coral-xyz/anchor/dist/cjs/nodewallet';
import {
  AddressLookupTableAccount,
  ComputeBudgetProgram,
  MessageV0,
  Signer,
  TransactionConfirmationStatus,
  TransactionError,
  TransactionInstruction,
  TransactionSignature,
  VersionedTransaction,
} from '@solana/web3.js';

export interface MangoSignatureStatus {
  slot: number;
  confirmations: number | null;
  err: TransactionError | null;
  confirmationStatus?: TransactionConfirmationStatus;
  signature: TransactionSignature;
}

export async function sendTransaction(
  provider: AnchorProvider,
  ixs: TransactionInstruction[],
  alts: AddressLookupTableAccount[],
  opts: any = {},
): Promise<MangoSignatureStatus> {
  const connection = provider.connection;
  const latestBlockhash =
    opts.latestBlockhash ??
    (await connection.getLatestBlockhash(
      opts.preflightCommitment ??
        provider.opts.preflightCommitment ??
        'finalized',
    ));

  const payer = (provider as AnchorProvider).wallet;

  if (opts.prioritizationFee) {
    ixs = [createComputeBudgetIx(opts.prioritizationFee), ...ixs];
  }

  const message = MessageV0.compile({
    payerKey: (provider as AnchorProvider).wallet.publicKey,
    instructions: ixs,
    recentBlockhash: latestBlockhash.blockhash,
    addressLookupTableAccounts: alts,
  });
  let vtx = new VersionedTransaction(message);
  if (opts?.additionalSigners?.length) {
    vtx.sign([...opts?.additionalSigners]);
  }

  if (
    typeof payer.signTransaction === 'function' &&
    !(payer instanceof NodeWallet || payer.constructor.name == 'NodeWallet')
  ) {
    vtx = (await payer.signTransaction(
      vtx as any,
    )) as unknown as VersionedTransaction;
  } else {
    // Maybe this path is only correct for NodeWallet?
    vtx.sign([(payer as any).payer as Signer]);
  }

  const signature = await connection.sendRawTransaction(vtx.serialize(), {
    skipPreflight: true, // mergedOpts.skipPreflight,
  });

  // const signature = await connection.sendTransactionss(
  //   vtx as any as VersionedTransaction,
  //   {
  //     skipPreflight: true,
  //   },
  // );

  if (opts.postSendTxCallback) {
    try {
      opts.postSendTxCallback({ txid: signature });
    } catch (e) {
      console.warn(`postSendTxCallback error ${e}`);
    }
  }

  const txConfirmationCommitment = opts.txConfirmationCommitment ?? 'processed';
  let status: any;
  if (
    latestBlockhash.blockhash != null &&
    latestBlockhash.lastValidBlockHeight != null
  ) {
    status = (
      await connection.confirmTransaction(
        {
          signature: signature,
          blockhash: latestBlockhash.blockhash,
          lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
        },
        txConfirmationCommitment,
      )
    ).value;
  } else {
    status = (
      await connection.confirmTransaction(signature, txConfirmationCommitment)
    ).value;
  }

  if (status.err) {
    console.warn('Tx status: ', status);
    throw new MangoError({
      txid: signature,
      message: `${JSON.stringify(status)}`,
    });
  }

  return { signature, ...status };
}

export const createComputeBudgetIx = (
  microLamports: number,
): TransactionInstruction => {
  const computeBudgetIx = ComputeBudgetProgram.setComputeUnitPrice({
    microLamports,
  });
  return computeBudgetIx;
};

class MangoError extends Error {
  message: string;
  txid: string;

  constructor({ txid, message }) {
    super();
    this.message = message;
    this.txid = txid;
  }
}
