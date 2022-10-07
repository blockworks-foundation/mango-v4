import { AnchorProvider } from '@project-serum/anchor';
import {
  AddressLookupTableAccount,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import { buildVersionedTx } from '../utils';

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

  let tx: Transaction = new Transaction();
  const altsEnabled = false;
  if (altsEnabled) {
    tx = await buildVersionedTx(provider, ixs, opts.additionalSigners, alts);
  } else {
    const payer = (provider as AnchorProvider).wallet;
    tx = new Transaction();
    tx.recentBlockhash = latestBlockhash.blockhash;
    tx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
    tx.feePayer = payer.publicKey;
    tx.add(...ixs);
    if (opts.additionalSigners?.length > 0) {
      tx.partialSign(...opts.additionalSigners);
    }
    await payer.signTransaction(tx);
  }

  const signature = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
  });

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
    // TODO: tyler, can we remove these?
    console.log('confirming via blockhash');
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
    // TODO: tyler, can we remove these?
    console.log('confirming via timeout');
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
