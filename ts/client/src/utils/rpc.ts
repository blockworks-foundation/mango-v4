import { AnchorProvider } from '@project-serum/anchor';
import {
  AddressLookupTableAccount,
  MessageV0,
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

  const message = MessageV0.compile({
    payerKey: (provider as AnchorProvider).wallet.publicKey,
    instructions: ixs,
    recentBlockhash: latestBlockhash.blockhash,
    addressLookupTableAccounts: alts,
  });
  const vtx = new VersionedTransaction(message);
  if (opts?.additionalSigners?.length) {
    vtx.sign([...opts?.additionalSigners]);
  }

  // if (payer instanceof Wallet) {
  const tx = await payer.signTransaction(vtx as any);
  // } else {
  //   tx.sign([((provider as AnchorProvider).wallet as any).payer as Signer]);
  // }

  const signature = await connection.sendTransaction(
    tx as any as VersionedTransaction,
    {
      skipPreflight: true,
    },
  );

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
    throw new MangoError({
      txid: signature,
      message: `Transaction ${signature} failed (${JSON.stringify(status)})`,
    });
  }

  return signature;
}

class MangoError extends Error {
  message: string;
  txid: string;

  constructor({ txid, message }) {
    super();
    this.message = message;
    this.txid = txid;
  }
}
