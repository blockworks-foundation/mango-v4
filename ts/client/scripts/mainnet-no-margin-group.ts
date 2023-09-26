import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../src/accounts/bank';
import { Group } from '../src/accounts/group';
import { MangoClient } from '../src/client';
import { DefaultTokenRegisterParams } from '../src/clientIxParamBuilder';
import { MANGO_V4_ID } from '../src/constants';
import { toNative } from '../src/utils';

const MANGO_PROGRAM = new PublicKey('4MTevvuuqC2sZtckP6RPeLZcqV3JyqoF3N5fswzc3NmT');

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'], // 0
  ['SOL', 'So11111111111111111111111111111111111111112'], // 1
]);
const MAINNET_ORACLES = new Map([
  ['USDC', 'Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD'],
  ['SOL', 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'],
]);
const MAINNET_SERUM3_MARKETS = new Map([
  ['SOL/USDC', '8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6']
]);
const {
  MB_CLUSTER_URL,
  MB_PAYER_KEYPAIR,
  GROUP_NUM,
}: {
  MB_CLUSTER_URL: string;
  MB_PAYER_KEYPAIR: string;
  GROUP_NUM: number;
} = process.env as any;

const defaultOracleConfig = {
  confFilter: 0.1,
  maxStalenessSlots: null,
};

const defaultInterestRate = {
  adjustmentFactor: 0.0,
  util0: 0.0,
  rate0: 0.0,
  util1: 0.0,
  rate1: 0.0,
  maxRate: 0.51,
};

const defaultTokenParams = {
  ...DefaultTokenRegisterParams,
  oracleConfig: defaultOracleConfig,
  interestRateParams: defaultInterestRate,
  loanOriginationFeeRate: 0.0,
  loanFeeRate: 0.0,
  initAssetWeight: 0,
  maintAssetWeight: 0,
  initLiabWeight: 1,
  maintLiabWeight: 1,
  liquidationFee: 0,
  minVaultToDepositsRatio: 1,
  netBorrowLimitPerWindowQuote: 0,
};

async function buildAdminClient(): Promise<[MangoClient, Keypair]> {
  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);
  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_PROGRAM,
    {
      idsSource: 'get-program-accounts',
    },
  );
  return [client, admin];
}

async function buildUserClient(): Promise<[MangoClient, Group, Keypair]> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR, 'utf-8'))),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);

  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_PROGRAM,
  );
  const group = await client.getGroupForCreator(user.publicKey, GROUP_NUM);
  return [client, group, user];
}

async function createGroup(): Promise<void> {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const insuranceMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  await client.groupCreate(GROUP_NUM, true, 2, insuranceMint);
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`...registered group ${group.publicKey}`);

  await client.groupEdit(
    group,
    undefined, // admin
    admin.publicKey, // fast listing
  );
  console.log(`...set fast listing admin`);
}

async function registerTokens(): Promise<void> {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  const usdcMainnetMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  const usdcMainnetOracle = new PublicKey(MAINNET_ORACLES.get('USDC')!);
  let sig = await client.tokenRegister(
    group,
    usdcMainnetMint,
    usdcMainnetOracle,
    0,
    'USDC',
    defaultTokenParams,
  );
  console.log(`registered usdc ${sig}`);

  const solMainnetMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
  const solMainnetOracle = new PublicKey(MAINNET_ORACLES.get('SOL')!);
  sig = await client.tokenRegisterTrustless(
    group,
    solMainnetMint,
    solMainnetOracle,
    1,
    'SOL',
  );
  console.log(`registered sol ${sig}`);
}

async function registerSerum3Market(): Promise<void> {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  await client.serum3RegisterMarket(
    group,
    new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
    group.getFirstBankByTokenIndex(1 as TokenIndex),
    group.getFirstBankByTokenIndex(0 as TokenIndex),
    0,
    'SOL/USDC',
  );
}

async function doUserAction(): Promise<void> {
  const result = await buildUserClient();
  const client = result[0];
  const group = result[1];
  const user = result[2];

  let mangoAccount = await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  );

  if (!mangoAccount) {
    await client.createMangoAccount(group, 0);
    mangoAccount = await client.getMangoAccountForOwner(
      group,
      user.publicKey,
      0,
    );
  }

  //   await client.tokenDeposit(
  //     group,
  //     mangoAccount!,
  //     new PublicKey(MAINNET_MINTS.get('SOL')!),
  //     0.01,
  //   );

  //   await client.tcsStopLossOnDeposit(
  //     group,
  //     mangoAccount!,
  //     group.getFirstBankByTokenIndex(1 as TokenIndex),
  //     group.getFirstBankByTokenIndex(0 as TokenIndex),
  //     group.getFirstBankByTokenIndex(1 as TokenIndex).uiPrice * 1.1,
  //     false,
  //     null,
  //     null,
  //     null,
  //   );

  await mangoAccount?.reload(client);
  mangoAccount
    ?.tokenConditionalSwapsActive()
    .map((tcs) => console.log(tcs.toString(group)));
}

async function doUserAction2(): Promise<void> {
  const result = await buildUserClient();
  const client = result[0];
  const group = result[1];
  const user = result[2];

  let mangoAccount = await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    1,
  );

  if (!mangoAccount) {
    await client.createMangoAccount(group, 1);
    mangoAccount = await client.getMangoAccountForOwner(
      group,
      user.publicKey,
      1,
    );
  }

  await client.tokenDeposit(
    group,
    mangoAccount!,
    new PublicKey(MAINNET_MINTS.get('USDC')!),
    5,
  );
}

async function placeAuction(): Promise<void> {
  const result = await buildUserClient();
  const client = result[0];
  const group = result[1];
  const user = result[2];

  const mangoAccount = await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    1,
  );
  console.log(mangoAccount?.publicKey.toString());

  const usdcBank = group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('USDC')!));
  const solBank = group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('SOL')!));

  await client.tokenConditionalSwapCancelAll(group, mangoAccount!);

  await client.tokenConditionalSwapCreateLinearAuction(
    group,
    mangoAccount!,
    usdcBank,
    solBank,
    1.0,
    30.0,
    0.001,
    Number.MAX_SAFE_INTEGER,
    true,
    false,
    true,
    Math.floor(Date.now() / 1000) + 120,
    180,
    null,
  );
}

async function main(): Promise<void> {
  try {
    // await createGroup();
    // await registerTokens();
    // await registerSerum3Market();
    // await doUserAction();
    // await doUserAction2();
    await placeAuction();
  } catch (error) {
    console.log(error);
  }
}

main();
