import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import * as path from 'path';
import { parse } from 'csv-parse';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { MANGO_V4_ID, MangoClient, toNative, USDC_MINT } from '../src';
import { WRAPPED_SOL_MINT } from '@project-serum/serum/lib/token-instructions';

const MANGO_MAINNET_GROUP = new PublicKey(
  '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX',
);

type Reimbursement = {
  mango_account: string;
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

const ownerOfMangoAccount = async (mangoAccount: string) => {
  try {
    const respWrapped = await fetch(
      `https://api.mngo.cloud/data/v4/user-data/profile-search?search-string=${mangoAccount}&search-method=mango-account`,
    );
    const resp = await respWrapped.json();
    const accountOwner = resp?.length > 0 ? resp[0].wallet_pk : null;
    if (accountOwner === null) {
      throw 'not found';
    }
    return accountOwner as string;
  } catch (e) {
    console.log(e);
    console.log('cant find mangoAccount:', mangoAccount);
  }
};

const readCsv = async () => {
  const csvFilePath = path.resolve(__dirname, 'reimbursement.csv');

  const headers = [
    'mango_account',
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

const main = async () => {
  const user = await setupWallet();
  const mainConnection = new Connection('http://fcs-da1._peer.internal:18899/');
  const backupConnections = [
    new Connection(
      'https://staked.helius-rpc.com?api-key=c367d0d9-acf2-437d-9ef3-72304f1a4117',
    ),
    new Connection('https://rpc.mngo.cloud/adr9quiemae0'),
  ];
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
  const TOKEN = 'MOTHER';

  const notReimbursedMangoAccounts: string[] = [];
  for (const row of TO_PROCESS) {
    const mangoAccountPk = tryGetPubKey(row.mango_account);

    if (mangoAccountPk) {
      try {
        const mint = mints[TOKEN as keyof typeof mints];
        const amount = Number(row[TOKEN as keyof typeof mints]);
        if (mint && amount > 0) {
          const mangoAccount = await client.getMangoAccount(mangoAccountPk);
          console.log('Mango Account exists');
          console.log(
            `Start reimbursing ${mint.toBase58()} ${amount} ${
              row.mango_account
            }`,
          );
          try {
            const decimals = group.getMintDecimals(mint);
            const nativeAmount = toNative(amount, decimals);

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
              notReimbursedMangoAccounts.push(row.mango_account);
            }
          } catch (e) {
            console.log(e);
            notReimbursedMangoAccounts.push(row.mango_account);
          }
        }
      } catch (e) {
        console.log('Mango account not exists', e);
        const wallet = await ownerOfMangoAccount(row.mango_account);
        if (!wallet) {
          notReimbursedMangoAccounts.push(row.mango_account);
        } else {
          notReimbursedMangoAccounts.push(row.mango_account);
          console.log('Mango Account: ', row.mango_account, 'Owner: ', wallet);
        }
      }
    } else {
      console.log('Invalid PublicKey: ', row.mango_account);
      throw 'Invalid PublicKey';
    }
  }
  console.log(notReimbursedMangoAccounts);
};

main();
