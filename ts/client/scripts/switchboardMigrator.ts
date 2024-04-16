// import { createComputeBudgetIx } from '@blockworks-foundation/mango-v4';
// import { PublicKey, chunk } from '@metaplex-foundation/js';
// import { bs58 } from '@project-serum/anchor/dist/cjs/utils/bytes';
// import { Connection, Keypair, Transaction } from '@solana/web3.js';
// import {
//   SwitchboardProgram,
//   QueueAccount,
//   CrankAccount,
//   AggregatorAccount,
// } from '@switchboard-xyz/solana.js';
// import { MANGO_DAO_WALLET } from './governance/constants';
// import { OracleJob } from '@switchboard-xyz/common';
// import { Wallet } from '@coral-xyz/anchor';
// import { awaitTransactionSignatureConfirmation } from '@blockworks-foundation/mangolana/lib/transactions';

// const newOracleName = 'NOS/USD';
// const oldOraclePk = '2FGoL9PNhNGpduRKLsTa4teRaX3vfarXAc1an2KyXxQm';

// const SWITCHBOARD_PERMISSIONLESS_QUE =
//   '5JYwqvKkqp35w8Nq3ba4z1WYUeJQ1rB36V8XvaGp6zn1';
// const SWITCHBOARD_PERMISSIONLESS_CRANK =
//   'BKtF8yyQsj3Ft6jb2nkfpEKzARZVdGgdEPs6mFmZNmbA';

// async function run() {
//   const WALLET = new Wallet(Keypair.fromSecretKey(bs58.decode('')));
//   const connection = new Connection('https://api.mngo.cloud/lite-rpc/v1/');
//   const program = await SwitchboardProgram.load(connection);
//   const payer = WALLET.publicKey;

//   const [[queueAccount], [crankAccount]] = await Promise.all([
//     QueueAccount.load(program, SWITCHBOARD_PERMISSIONLESS_QUE),
//     CrankAccount.load(program, SWITCHBOARD_PERMISSIONLESS_CRANK),
//   ]);

//   const [aggregatorAccountOld, aggregatorAccountDataOld] =
//     await AggregatorAccount.load(program, new PublicKey(oldOraclePk));

//   const jobs = await aggregatorAccountOld.loadJobs(aggregatorAccountDataOld);
//   const newJobs: string[] = [];
//   for (const job of jobs) {
//     const jobYaml = job.job.toYaml();
//     const remove_after = jobYaml.indexOf('- multiplyTask');
//     let result = jobYaml.substring(0, remove_after);
//     result =
//       result +
//       `- conditionalTask:
//       attempt:
//         - multiplyTask:
//             job:
//               tasks:
//                 - oracleTask:
//                     pythAddress: Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD
//                     pythAllowedConfidenceInterval: 10
//       onFailure:
//         - multiplyTask:
//             job:
//               tasks:
//                 - oracleTask:
//                     switchboardAddress: FwYfsmj5x8YZXtQBNo2Cz8TE7WRCMFqA6UTffK4xQKMH`;
//     newJobs.push(result);
//   }

//   const [aggregatorAccountNew, txArray1] =
//     await queueAccount.createFeedInstructions(payer, {
//       name: newOracleName,
//       batchSize: aggregatorAccountDataOld.oracleRequestBatchSize,
//       minRequiredOracleResults: aggregatorAccountDataOld.minOracleResults,
//       minRequiredJobResults: aggregatorAccountDataOld.minJobResults,
//       minUpdateDelaySeconds: aggregatorAccountDataOld.minUpdateDelaySeconds,
//       forceReportPeriod: 60 * 60,
//       withdrawAuthority: MANGO_DAO_WALLET,
//       authority: payer,
//       crankDataBuffer: crankAccount.dataBuffer?.publicKey,
//       crankPubkey: crankAccount.publicKey,
//       fundAmount: 0.1,
//       slidingWindow: true,
//       disableCrank: false,
//       maxPriorityFeeMultiplier: 5,
//       priorityFeeBumpPeriod: 10,
//       priorityFeeBump: 1000,
//       basePriorityFee: 1000,
//       jobs: [
//         ...newJobs.map((x) => ({
//           weight: 1,
//           data: OracleJob.encodeDelimited(OracleJob.fromYaml(x)).finish(),
//         })),
//       ],
//     });

//   const lockTx = aggregatorAccountNew.lockInstruction(payer, {});
//   const transferAuthIx = aggregatorAccountNew.setAuthorityInstruction(payer, {
//     newAuthority: MANGO_DAO_WALLET,
//   });
//   const latestBlockhash = await connection.getLatestBlockhash('processed');

//   const txChunks = chunk([...txArray1, lockTx, transferAuthIx], 1);

//   const transactions: Transaction[] = [];

//   for (const chunkIndex in txChunks) {
//     const chunk = txChunks[chunkIndex];
//     const tx = new Transaction();
//     const singers = [...chunk.flatMap((x) => x.signers)];
//     tx.add(createComputeBudgetIx(800000));
//     tx.add(...chunk.flatMap((x) => x.ixns));
//     tx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
//     tx.recentBlockhash = latestBlockhash.blockhash;
//     tx.feePayer = payer;
//     if (singers.length) {
//       tx.sign(...singers);
//     }
//     transactions.push(tx);
//   }
//   const signedTxes = await WALLET.signAllTransactions(transactions);

//   for (const signed of signedTxes) {
//     const rawTransaction = signed.serialize();
//     const signature = await connection.sendRawTransaction(rawTransaction, {
//       skipPreflight: true,
//     });
//     await awaitTransactionSignatureConfirmation({
//       txid: signature,
//       confirmLevel: 'processed',
//       connection: connection,
//       timeoutStrategy: {
//         block: latestBlockhash,
//       },
//       config: {
//         logFlowInfo: true,
//       },
//     });
//   }

//   console.log(aggregatorAccountNew.publicKey.toBase58(), '@@@@@');
// }
// try {
//   run();
// } catch (e) {
//   console.log(e);
// }
