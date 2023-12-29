import { AnchorProvider } from '@coral-xyz/anchor';
import NodeWallet from '@coral-xyz/anchor/dist/cjs/nodewallet';
import { u8 } from '@solana/buffer-layout';
import {
  AddressLookupTableAccount,
  Commitment,
  ComputeBudgetProgram,
  Connection,
  Keypair,
  MessageV0,
  RpcResponseAndContext,
  SignatureResult,
  Signer,
  TransactionConfirmationStatus,
  TransactionError,
  TransactionInstruction,
  TransactionSignature,
  VersionedTransaction,
} from '@solana/web3.js';
import { Tracing } from 'trace_events';
import { COMPUTE_BUDGET_PROGRAM_ID } from '../constants';

export interface MangoSignatureStatus {
  confirmations?: number | null;
  confirmationStatus?: TransactionConfirmationStatus;
  err: TransactionError | null;
  signature: TransactionSignature;
  slot: number;
}

export interface MangoSignature {
  signature: TransactionSignature;
}

export type SendTransactionOpts = Partial<{
  preflightCommitment: Commitment;
  latestBlockhash: Readonly<{
    blockhash: string;
    lastValidBlockHeight: number;
  }>;
  prioritizationFee: number;
  estimateFee: boolean;
  additionalSigners: Keypair[];
  postSendTxCallback: ({ txid }: { txid: string }) => void;
  postTxConfirmationCallback: ({ txid }: { txid: string }) => void;
  txConfirmationCommitment: Commitment;
  confirmInBackground: boolean;
  alts: AddressLookupTableAccount[];
  multipleConnections: Connection[];
}>;

export function sendTransaction(
  provider: AnchorProvider,
  ixs: TransactionInstruction[],
  alts: AddressLookupTableAccount[],
  opts?: { confirmInBackground: true } & SendTransactionOpts,
): Promise<MangoSignature>;

export function sendTransaction(
  provider: AnchorProvider,
  ixs: TransactionInstruction[],
  alts: AddressLookupTableAccount[],
  opts?: SendTransactionOpts,
): Promise<MangoSignatureStatus>;

export async function sendTransaction(
  provider: AnchorProvider,
  ixs: TransactionInstruction[],
  alts: AddressLookupTableAccount[],
  opts: SendTransactionOpts = {},
): Promise<MangoSignatureStatus | MangoSignature> {
  const connection = provider.connection;
  const latestBlockhash =
    opts.latestBlockhash ??
    (await connection.getLatestBlockhash(
      opts.preflightCommitment ??
        provider.opts.preflightCommitment ??
        'finalized',
    ));

  const payer = (provider as AnchorProvider).wallet;

  //
  // setComputeUnitLimit, hard code to a higher minimum, this is needed so that we dont fail simple UI interactions
  //
  // https://github.com/solana-labs/solana-web3.js/blob/master/packages/library-legacy/src/programs/compute-budget.ts#L202
  const computeUnitLimitIxFound = ixs.some(
    (ix) =>
      ix.programId.equals(COMPUTE_BUDGET_PROGRAM_ID) &&
      u8().decode(ix.data.subarray(0, 1)) == 2,
  );

  if (!computeUnitLimitIxFound) {
    const totalUserIntendedIxs = ixs.filter(
      (ix) => !ix.programId.equals(COMPUTE_BUDGET_PROGRAM_ID),
    ).length;
    const requestCu = Math.min(totalUserIntendedIxs * 300_000, 1_600_000);
    ixs = [
      ComputeBudgetProgram.setComputeUnitLimit({
        units: requestCu,
      }),
      ...ixs,
    ];
  }

  //
  // setComputeUnitPrice
  //
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

  // if configured, send the transaction using multiple connections
  let signature: string;
  if (opts?.multipleConnections?.length ?? 0 > 0) {
    const allConnections = [connection, ...opts.multipleConnections!];
    signature = await Promise.any(
      allConnections.map((c) => {
        return c.sendRawTransaction(vtx.serialize(), {
          skipPreflight: true, // mergedOpts.skipPreflight,
        });
      }),
    );
  } else {
    signature = await connection.sendRawTransaction(vtx.serialize(), {
      skipPreflight: true, // mergedOpts.skipPreflight,
    });
  }

  if (opts.postSendTxCallback) {
    try {
      opts.postSendTxCallback({ txid: signature });
    } catch (e) {
      console.warn(`postSendTxCallback error ${e}`);
    }
  }

  if (!opts.confirmInBackground) {
    return await confirmTransaction(
      connection,
      opts,
      latestBlockhash,
      signature,
    );
  } else {
    confirmTransaction(connection, opts, latestBlockhash, signature);
    return { signature };
  }
}

const confirmTransaction = async (
  connection: Connection,
  opts: Partial<SendTransactionOpts> = {},
  latestBlockhash: Readonly<{
    blockhash: string;
    lastValidBlockHeight: number;
  }>,
  signature: string,
): Promise<MangoSignatureStatus> => {
  const txConfirmationCommitment = opts.txConfirmationCommitment ?? 'processed';
  let status: RpcResponseAndContext<SignatureResult>;
  if (
    latestBlockhash.blockhash != null &&
    latestBlockhash.lastValidBlockHeight != null
  ) {
    status = await connection.confirmTransaction(
      {
        signature: signature,
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      },
      txConfirmationCommitment,
    );
  } else {
    status = await connection.confirmTransaction(
      signature,
      txConfirmationCommitment,
    );
  }
  const signatureResult = status.value;
  if (signatureResult.err) {
    console.warn('Tx status: ', status);
    throw new MangoError({
      txid: signature,
      message: `${JSON.stringify(status)}`,
    });
  }
  if (opts.postTxConfirmationCallback) {
    try {
      opts.postTxConfirmationCallback({ txid: signature });
    } catch (e) {
      console.warn(`postTxConfirmationCallback error ${e}`);
    }
  }
  return { signature, slot: status.context.slot, ...signatureResult };
};

export const createComputeBudgetIx = (
  microLamports: number,
): TransactionInstruction => {
  const computeBudgetIx = ComputeBudgetProgram.setComputeUnitPrice({
    microLamports,
  });
  return computeBudgetIx;
};

export class MangoError extends Error {
  message: string;
  txid: string;

  constructor({ txid, message }) {
    super();
    this.message = message;
    this.txid = txid;
  }
}
