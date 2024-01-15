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
import { COMPUTE_BUDGET_PROGRAM_ID } from '../constants';
import { TxCallbackOptions } from '../client';
import { awaitTransactionSignatureConfirmation } from '@blockworks-foundation/mangolana/lib/transactions';
import { tryStringify } from '../utils';

export interface MangoSignatureStatus {
  confirmations?: number | null;
  confirmationStatus?: TransactionConfirmationStatus;
  err?: TransactionError | null;
  signature: TransactionSignature;
  slot?: number;
}

export interface LatestBlockhash {
  slot: number;
  blockhash: string;
  lastValidBlockHeight: number;
}

export interface LatestBlockhash {
  slot: number;
  blockhash: string;
  lastValidBlockHeight: number;
}

export type SendTransactionOpts = Partial<{
  preflightCommitment: Commitment;
  latestBlockhash: Readonly<LatestBlockhash>;
  prioritizationFee: number;
  estimateFee: boolean;
  additionalSigners: Keypair[];
  postSendTxCallback: (callbackOpts: TxCallbackOptions) => void;
  postTxConfirmationCallback: (callbackOpts: TxCallbackOptions) => void;
  txConfirmationCommitment: Commitment;
  confirmInBackground: boolean;
  alts: AddressLookupTableAccount[];
  multipleConnections: Connection[];
}>;

export async function sendTransaction(
  provider: AnchorProvider,
  ixs: TransactionInstruction[],
  alts: AddressLookupTableAccount[],
  opts: SendTransactionOpts = {},
): Promise<MangoSignatureStatus> {
  const connection = provider.connection;
  const latestBlockhash = await fetchLatestBlockHash(provider, opts);

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
      opts.postSendTxCallback({
        txid: signature,
        txSignatureBlockHash: latestBlockhash,
      });
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
  latestBlockhash: Readonly<LatestBlockhash>,
  signature: string,
): Promise<MangoSignatureStatus> => {
  let status: RpcResponseAndContext<SignatureResult>;
  const allConnections = [connection];
  if (opts.multipleConnections && opts.multipleConnections.length) {
    allConnections.push(...opts.multipleConnections);
  }
  const abortController = new AbortController();
  try {
    if (
      latestBlockhash.blockhash != null &&
      latestBlockhash.lastValidBlockHeight != null
    ) {
      status = await Promise.any(
        allConnections.map((c) =>
          awaitTransactionSignatureConfirmation({
            txid: signature,
            confirmLevel: 'processed',
            connection: c,
            timeoutStrategy: {
              block: latestBlockhash,
            },
            abortSignal: abortController.signal,
          }),
        ),
      );
    } else {
      status = await Promise.any(
        allConnections.map((c) =>
          awaitTransactionSignatureConfirmation({
            txid: signature,
            confirmLevel: 'processed',
            connection: c,
            timeoutStrategy: {
              timeout: 90,
            },
            abortSignal: abortController.signal,
          }),
        ),
      );
    }
    abortController.abort();

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
        opts.postTxConfirmationCallback({
          txid: signature,
          txSignatureBlockHash: latestBlockhash,
        });
      } catch (e) {
        console.warn(`postTxConfirmationCallback error ${e}`);
      }
    }
    return { signature, slot: status.context.slot, ...signatureResult };
  } catch (e) {
    abortController.abort();
    if (e instanceof AggregateError) {
      for (const individualError of e.errors) {
        const stringifiedError = tryStringify(individualError);
        throw new MangoError({
          txid: signature,
          message: `${
            stringifiedError
              ? stringifiedError
              : individualError
              ? individualError
              : 'Unknown error'
          }`,
        });
      }
    }
    if (isErrorWithSignatureResult(e)) {
      const stringifiedError = tryStringify(e?.value?.err);
      throw new MangoError({
        txid: signature,
        message: `${stringifiedError ? stringifiedError : e?.value?.err}`,
      });
    }
    const stringifiedError = tryStringify(e);
    throw new MangoError({
      txid: signature,
      message: `${stringifiedError ? stringifiedError : e}`,
    });
  }
};

export async function fetchLatestBlockHash(
  provider: AnchorProvider,
  opts: SendTransactionOpts = {},
): Promise<LatestBlockhash> {
  if (opts.latestBlockhash) {
    return opts.latestBlockhash;
  }
  const commitment =
    opts.preflightCommitment ??
    provider.opts.preflightCommitment ??
    'finalized';
  const blockhashRequest =
    await provider.connection.getLatestBlockhashAndContext(commitment);
  return {
    slot: blockhashRequest.context.slot,
    lastValidBlockHeight: blockhashRequest.value.lastValidBlockHeight,
    blockhash: blockhashRequest.value.blockhash,
  };
}

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

function isErrorWithSignatureResult(
  err: any,
): err is RpcResponseAndContext<SignatureResult> {
  return err && typeof err.value !== 'undefined';
}
