import {
  createApproveInstruction,
  createCloseAccountInstruction,
  createSyncNativeInstruction,
  createTransferInstruction,
  getAccount,
  getAssociatedTokenAddress,
  NATIVE_MINT,
} from '@solana/spl-token';
import {
  Connection,
  Keypair,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
} from '@solana/web3.js';
import fs from 'fs';

async function main(): Promise<void> {
  try {
    let sig;
    const conn = new Connection(process.env.MB_CLUSTER_URL!);

    // load wallet 1
    const w1 = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(process.env.wallet1!, 'utf-8'))),
    );

    // load wallet 2
    const w2 = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(process.env.wallet2!, 'utf-8'))),
    );

    const w1WsolTA = await getAssociatedTokenAddress(NATIVE_MINT, w1.publicKey);
    //   const ataTransaction1 = new Transaction().add(
    //     createAssociatedTokenAccountInstruction(
    //       w1.publicKey,
    //       w1WsolTA,
    //       w1.publicKey,
    //       NATIVE_MINT,
    //     ),
    //   );
    //   await sendAndConfirmTransaction(conn, ataTransaction1, [w1]);

    const w2WsolTA = await getAssociatedTokenAddress(NATIVE_MINT, w2.publicKey);
    //   const ataTransaction2 = new Transaction().add(
    //     createAssociatedTokenAccountInstruction(
    //       w2.publicKey,
    //       w2WsolTA,
    //       w2.publicKey,
    //       NATIVE_MINT,
    //     ),
    //   );
    //   await sendAndConfirmTransaction(conn, ataTransaction2, [w2]);

    // wallet 1 wrap sol to wsol
    const solTransferTransaction = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: w1.publicKey,
        toPubkey: w1WsolTA,
        lamports: 1,
      }),
      createSyncNativeInstruction(w1WsolTA),
    );
    sig = await sendAndConfirmTransaction(conn, solTransferTransaction, [w1]);
    console.log(
      `sig w1 wrapped some sol https://explorer.solana.com/tx/${sig}`,
    );

    // wallet 1 approve wallet 2 for some wsol
    const tokenApproveTx = new Transaction().add(
      createApproveInstruction(w1WsolTA, w2.publicKey, w1.publicKey, 1),
    );
    sig = await sendAndConfirmTransaction(conn, tokenApproveTx, [w1]);
    console.log(
      `sig w1 token approve w2 https://explorer.solana.com/tx/${sig}`,
    );

    // log delegate amount
    let w2WsolAtaInfo = await getAccount(conn, w1WsolTA);
    console.log(
      `- delegate ${w2WsolAtaInfo.delegate}, amount ${w2WsolAtaInfo.delegatedAmount}`,
    );

    // wallet 2 transfer wsol from wallet 1 to wallet 2
    const tokenTransferTx = new Transaction().add(
      createTransferInstruction(w1WsolTA, w2WsolTA, w2.publicKey, 1),
    );
    sig = await sendAndConfirmTransaction(conn, tokenTransferTx, [w2], {
      skipPreflight: true,
    });
    console.log(
      `sig w1 transfer wsol to w2 https://explorer.solana.com/tx/${sig}`,
    );

    // log delegate amount
    w2WsolAtaInfo = await getAccount(conn, w1WsolTA, 'finalized');
    console.log(
      `- delegate ${w2WsolAtaInfo.delegate}, amount ${w2WsolAtaInfo.delegatedAmount}`,
    );

    // wallet 2 unwrap all wsol
    const closeAtaIx = new Transaction().add(
      createCloseAccountInstruction(w2WsolTA, w2.publicKey, w2.publicKey),
    );
    sig = await sendAndConfirmTransaction(conn, closeAtaIx, [w2], {
      skipPreflight: true,
    });
    console.log(`sig w2 unwrap wsol https://explorer.solana.com/tx/${sig}`);
  } catch (error) {
    console.log(error);
  }
}

main();
