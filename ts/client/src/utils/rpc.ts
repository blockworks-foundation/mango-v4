import { AnchorProvider } from '@project-serum/anchor';
import { Transaction } from '@solana/web3.js';

export async function sendTransaction(
  provider: AnchorProvider,
  transaction: Transaction,
  opts: any = {},
) {
  const connection = provider.connection;
  const payer = provider.wallet;
  transaction.recentBlockhash = (
    await connection.getLatestBlockhash(opts.preflightCommitment)
  ).blockhash;
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

  const status =
    transaction.recentBlockhash != null &&
    transaction.lastValidBlockHeight != null
      ? (
          await connection.confirmTransaction(
            {
              signature: signature,
              blockhash: transaction.recentBlockhash,
              lastValidBlockHeight: transaction.lastValidBlockHeight,
            },
            // options && options.commitment,
          )
        ).value
      : (
          await connection.confirmTransaction(
            signature,
            // options && options.commitment,
          )
        ).value;

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
