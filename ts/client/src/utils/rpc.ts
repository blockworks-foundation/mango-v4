import { AnchorProvider } from '@project-serum/anchor';
import { Transaction } from '@solana/web3.js';

export async function sendTransaction(
  provider: AnchorProvider,
  transaction: Transaction,
  opts: any = {},
) {
  const connection = provider.connection;
  const payer = provider.wallet;
  const latestBlockhash = await connection.getLatestBlockhash(
    opts.preflightCommitment,
  );
  transaction.recentBlockhash = latestBlockhash.blockhash;
  transaction.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
  transaction.feePayer = payer.publicKey;
  if (opts.additionalSigners?.length > 0) {
    transaction.partialSign(...opts.additionalSigners);
  }

  await payer.signTransaction(transaction);
  const rawTransaction = transaction.serialize();

  const signature = await connection.sendRawTransaction(rawTransaction, {
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
    transaction.recentBlockhash != null &&
    transaction.lastValidBlockHeight != null
  ) {
    console.log('confirming via blockhash');
    status = (
      await connection.confirmTransaction(
        {
          signature: signature,
          blockhash: transaction.recentBlockhash,
          lastValidBlockHeight: transaction.lastValidBlockHeight,
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
