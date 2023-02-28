import NodeWallet from '@coral-xyz/anchor/dist/cjs/nodewallet';
import { ComputeBudgetProgram, MessageV0, VersionedTransaction, } from '@solana/web3.js';
export async function sendTransaction(provider, ixs, alts, opts = {}) {
    const connection = provider.connection;
    const latestBlockhash = await connection.getLatestBlockhash(opts.preflightCommitment ??
        provider.opts.preflightCommitment ??
        'finalized');
    const payer = provider.wallet;
    if (opts.prioritizationFee) {
        ixs = [createComputeBudgetIx(opts.prioritizationFee), ...ixs];
    }
    const message = MessageV0.compile({
        payerKey: provider.wallet.publicKey,
        instructions: ixs,
        recentBlockhash: latestBlockhash.blockhash,
        addressLookupTableAccounts: alts,
    });
    let vtx = new VersionedTransaction(message);
    if (opts?.additionalSigners?.length) {
        vtx.sign([...opts?.additionalSigners]);
    }
    if (typeof payer.signTransaction === 'function' &&
        !(payer instanceof NodeWallet)) {
        vtx = (await payer.signTransaction(vtx));
    }
    else {
        // Maybe this path is only correct for NodeWallet?
        vtx.sign([payer.payer]);
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
        }
        catch (e) {
            console.warn(`postSendTxCallback error ${e}`);
        }
    }
    const txConfirmationCommitment = opts.txConfirmationCommitment ?? 'processed';
    let status;
    if (latestBlockhash.blockhash != null &&
        latestBlockhash.lastValidBlockHeight != null) {
        status = (await connection.confirmTransaction({
            signature: signature,
            blockhash: latestBlockhash.blockhash,
            lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
        }, txConfirmationCommitment)).value;
    }
    else {
        status = (await connection.confirmTransaction(signature, txConfirmationCommitment)).value;
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
export const createComputeBudgetIx = (microLamports) => {
    const computeBudgetIx = ComputeBudgetProgram.setComputeUnitPrice({
        microLamports,
    });
    return computeBudgetIx;
};
class MangoError extends Error {
    message;
    txid;
    constructor({ txid, message }) {
        super();
        this.message = message;
        this.txid = txid;
    }
}
