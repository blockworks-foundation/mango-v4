import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import {
  AddressLookupTableAccount,
  ComputeBudgetProgram,
  MessageV0,
  Signer,
  TransactionInstruction,
  VersionedTransaction,
} from '@solana/web3.js';

export async function sendTransaction(
  provider: AnchorProvider,
  ixs: TransactionInstruction[],
  alts: AddressLookupTableAccount[],
  opts: any = {},
): Promise<string> {
  const connection = provider.connection;
  const latestBlockhash = await connection.getLatestBlockhash(
    opts.preflightCommitment,
  );

  const payer = (provider as AnchorProvider).wallet;

  if (opts.prioritizationFee) {
    ixs = [
      createComputeBudgetIx(opts.prioritizationFee, 200_000 * ixs.length + 1),
      ...ixs,
    ];
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
    !(payer instanceof NodeWallet)
  ) {
    vtx = (await payer.signTransaction(
      vtx as any,
    )) as unknown as VersionedTransaction;
  } else {
    // Maybe this path is only correct for NodeWallet?
    vtx.sign([(payer as any).payer as Signer]);
  }

  const signature = await connection.sendRawTransaction(vtx.serialize(), {
    skipPreflight: true,
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
        'processed',
      )
    ).value;
  } else {
    status = (await connection.confirmTransaction(signature, 'processed'))
      .value;
  }

  if (status.err) {
    console.warn('Tx status: ', status);
    throw new MangoError({
      txid: signature,
      message: `${JSON.stringify(status)}`,
    });
  }

  return signature;
}

export const createComputeBudgetIx = (
  prioritizationFee: number,
  units: number,
): TransactionInstruction => {
  const computeBudgetIx = ComputeBudgetProgram.requestUnits({
    additionalFee: prioritizationFee,
    units,
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
