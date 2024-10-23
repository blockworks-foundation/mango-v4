import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';
import fs from 'fs';
import * as path from 'path';
import { parse } from 'csv-parse';
import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
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
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createTransferInstruction,
  getAssociatedTokenAddressSync,
} from '@solana/spl-token';

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

const sendTokenDeposit = (
  owner: string,
  wallet: Wallet,
  nativeAmount: BN,
  connection: Connection,
) => {
  const userAta = getAssociatedTokenAddressSync(
    USDC_MINT,
    new PublicKey(owner),
    true,
  );
  const myAta = getAssociatedTokenAddressSync(
    USDC_MINT,
    wallet.publicKey,
    true,
  );
  const createAtaIx = createAssociatedTokenAccountIdempotentInstruction(
    wallet.publicKey,
    userAta,
    new PublicKey(owner),
    USDC_MINT,
  );
  const sendIx = createTransferInstruction(
    myAta,
    userAta,
    wallet.publicKey,
    nativeAmount.toNumber(),
  );
  return sendSignAndConfirmTransactions({
    connection: connection,
    wallet: wallet,
    transactionInstructions: [
      {
        instructionsSet: [
          new TransactionInstructionWithSigners(
            ComputeBudgetProgram.setComputeUnitLimit({
              units: 40000,
            }),
          ),
          new TransactionInstructionWithSigners(createComputeBudgetIx(2000000)),
          new TransactionInstructionWithSigners(createAtaIx),
          new TransactionInstructionWithSigners(sendIx),
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
      prioritizationFee: 2000000,
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
      const decimals = group.getMintDecimals(mint);
      const nativeAmount = toNative(amount, decimals);
      try {
        if (mint && amount > 0.05) {
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
              await sendTokenDeposit(
                row.owner,
                userWallet,
                nativeAmount,
                userProvider.connection,
              );
            }
          } catch (e) {
            console.log(e);
            notReimbursedMangoAccounts.push(row.mango_account);
          }
        }
      } catch (e) {
        console.log('Mango account not exists', e);
        await sendTokenDeposit(
          row.owner,
          userWallet,
          nativeAmount,
          userProvider.connection,
        );
      }
    } else {
      console.log('Invalid PublicKey: ', row.mango_account);
      throw 'Invalid PublicKey';
    }
  }
  console.log(notReimbursedMangoAccounts);
};

main();
