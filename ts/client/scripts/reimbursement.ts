import { Connection, Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import fs from 'fs';
import * as path from 'path';
import { parse } from 'csv-parse';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  createComputeBudgetIx,
  MANGO_V4_ID,
  MangoClient,
  toNative,
  USDC_MINT,
} from '../src';
import { WRAPPED_SOL_MINT } from '@project-serum/serum/lib/token-instructions';
import { sendSignAndConfirmTransactions } from '@blockworks-foundation/mangolana/lib/transactions';
import {
  SequenceType,
  TransactionInstructionWithSigners,
} from '@blockworks-foundation/mangolana/lib/globalTypes';

const MANGO_MAINNET_GROUP = new PublicKey(
  '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX',
);

type Reimbursement = {
  mango_account: string;
  owner: string;
  mangoSOL: number;
  MOTHER: number;
  SOL: number;
  USDC: number;
  Notional: string;
};

const setupWallet = () => {
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync('keypair.json', {
          encoding: 'utf-8',
        }),
      ),
    ),
  );

  return user;
};

const readCsv = async () => {
  const csvFilePath = path.resolve(__dirname, 'reimbursement.csv');

  const headers = [
    'mango_account',
    'owner',
    'mangoSOL',
    'MOTHER',
    'SOL',
    'USDC',
    'Notional',
  ];

  return new Promise<Reimbursement[]>((resolve, reject) => {
    const fileContent = fs.readFileSync(csvFilePath, { encoding: 'utf-8' });

    parse(
      fileContent,
      {
        delimiter: ',',
        columns: headers,
      },
      (error, result: Reimbursement[]) => {
        if (error) {
          reject(error);
        } else {
          const resp = result.slice(1, result.length);
          resolve(resp);
        }
      },
    );
  });
};

const tryGetPubKey = (pubkey: string | string[]) => {
  try {
    return new PublicKey(pubkey);
  } catch (e) {
    console.log(e);
    return null;
  }
};

const mints = {
  mangoSOL: new PublicKey('MangmsBgFqJhW4cLUR9LxfVgMboY1xAoP8UUBiWwwuY'),
  MOTHER: new PublicKey('3S8qX1MsMqRbiwKg2cQyx7nis1oHMgaCuc9c4VfvVdPN'),
  SOL: WRAPPED_SOL_MINT,
  USDC: USDC_MINT,
};
const backups = [new Connection(''), new Connection('')];

const main = async () => {
  const user = await setupWallet();
  const mainConnection = new Connection('');
  const backupConnections = backups;
  const options = AnchorProvider.defaultOptions();
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(mainConnection, userWallet, options);

  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'api',
      multipleConnections: backupConnections,
      prioritizationFee: 200000,
    },
  );
  console.log(userWallet.publicKey.toBase58(), '@@@@@');
  const group = await client.getGroup(MANGO_MAINNET_GROUP);

  const csvData = await readCsv();

  const TO_PROCESS = csvData;
  const TOKEN = 'SOL';

  const notReimbursedMangoAccounts: string[] = [];
  for (const row of TO_PROCESS) {
    const mangoAccountPk = tryGetPubKey(row.mango_account);

    if (mangoAccountPk) {
      const mint = mints[TOKEN as keyof typeof mints];
      const amount = Number(row[TOKEN as keyof typeof mints]);
      try {
        if (mint && amount > 0.0001) {
          const decimals = group.getMintDecimals(mint);
          const nativeAmount = toNative(amount, decimals);
          const mangoAccount = await client.getMangoAccount(mangoAccountPk);
          console.log('Mango Account exists');
          console.log(
            `Start reimbursing ${mint.toBase58()} ${amount} ${
              row.mango_account
            }`,
          );
          try {
            const signature = await client.tokenDepositNative(
              group,
              mangoAccount,
              mint,
              nativeAmount,
              false,
              true,
            );

            console.log(
              'Reimburse end ',
              signature.signature,
              signature.confirmationStatus,
              signature.err,
            );
            if (!signature.err) {
              console.log('OK');
            } else {
              const ix = SystemProgram.transfer({
                fromPubkey: userWallet.publicKey,
                toPubkey: new PublicKey(row.owner),
                lamports: toNative(amount, 9).toNumber(),
              });
              await sendSignAndConfirmTransactions({
                connection: userProvider.connection,
                wallet: userWallet,
                transactionInstructions: [
                  {
                    instructionsSet: [
                      new TransactionInstructionWithSigners(
                        createComputeBudgetIx(200000),
                      ),
                      new TransactionInstructionWithSigners(ix),
                    ],
                    sequenceType: SequenceType.Sequential,
                  },
                ],
                backupConnections: [...backups],
                config: {
                  maxTxesInBatch: 2,
                  autoRetry: true,
                  logFlowInfo: true,
                },
              });
            }
          } catch (e) {
            console.log(e);
            notReimbursedMangoAccounts.push(row.mango_account);
          }
        }
      } catch (e) {
        console.log('Mango account not exists', e);
        const ix = SystemProgram.transfer({
          fromPubkey: userWallet.publicKey,
          toPubkey: new PublicKey(row.owner),
          lamports: toNative(amount, 9).toNumber(),
        });
        await sendSignAndConfirmTransactions({
          connection: userProvider.connection,
          wallet: userWallet,
          transactionInstructions: [
            {
              instructionsSet: [
                new TransactionInstructionWithSigners(
                  createComputeBudgetIx(200000),
                ),
                new TransactionInstructionWithSigners(ix),
              ],
              sequenceType: SequenceType.Sequential,
            },
          ],
          backupConnections: [...backups],
          config: {
            maxTxesInBatch: 2,
            autoRetry: true,
            logFlowInfo: true,
          },
        });
      }
    } else {
      console.log('Invalid PublicKey: ', row.mango_account);
      throw 'Invalid PublicKey';
    }
  }
  console.log(notReimbursedMangoAccounts);
};

main();
