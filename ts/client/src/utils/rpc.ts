import { AnchorProvider } from '@project-serum/anchor';
import {
  AddressLookupTableAccount,
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
  const tx = await buildVersionedTx(
    provider,
    ixs,
    opts.additionalSigners,
    alts,
  );

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
