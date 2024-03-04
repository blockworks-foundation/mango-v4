import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  AddressLookupTableProgram,
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  SystemProgram,
} from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../../src/accounts/bank';
import { Group } from '../../src/accounts/group';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../../src/accounts/serum3';
import { Builder } from '../../src/builder';
import { MangoClient } from '../../src/client';
import {
  DefaultTokenRegisterParams,
  NullPerpEditParams,
  NullTokenEditParams,
} from '../../src/clientIxParamBuilder';
import { MANGO_V4_ID, OPENBOOK_PROGRAM_ID } from '../../src/constants';
import { buildVersionedTx, toNative } from '../../src/utils';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from '../../src/utils/spl';

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'], // 0
  ['USDT', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB'], // 1
  ['DAI', 'EjmyN6qEC1Tf1JxiG1ae7UTJhUxSwk1TCWNWqxWV4J6o'], // 2
  ['ETH', '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'], // 3 Ether (Portal)
  ['SOL', 'So11111111111111111111111111111111111111112'], // 4 Wrapped SOL
  ['MSOL', 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So'], // 5
  ['MNGO', 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac'], // 6
  ['BONK', 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263'], // 7
]);
const MAINNET_ORACLES = new Map([
  // USDC - stub oracle
  ['USDT', '3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL'],
  ['DAI', 'CtJ8EkqLmeYyGB8s4jevpeNsvmD4dxVR2krfsDLcvV8Y'],
  ['ETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['SOL', 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'],
  ['MSOL', 'E4v1BBgoso9s64TQvmyownAVJbhbEPGyzA3qn4n46qj9'],
  // ['MNGO', '79wm3jjcPr6RaNQ4DGvP5KxG1mNd3gEBsg6FsNVFezK4'], // pyth
  ['MNGO', '5xUoyPG9PeowJvfai5jD985LiRvo58isaHrmmcBohi3Y'], // switchboard
  ['BTC', 'GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'],
  ['BONK', '4SZ1qb4MtSUrZcoeaeQ3BDzVCyqxw3VwSFpPiMTmn4GE'],
]);

// External markets are matched with those in https://github.com/openbook-dex/openbook-ts/blob/master/packages/serum/src/markets.json
const MAINNET_SERUM3_MARKETS = new Map([
  ['SOL/USDC', '8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6'],
]);

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, MB_PAYER3_KEYPAIR } = process.env;

const MIN_VAULT_TO_DEPOSITS_RATIO = 0.2;
const NET_BORROWS_WINDOW_SIZE_TS = 24 * 60 * 60;
const NET_BORROW_LIMIT_PER_WINDOW_QUOTE = toNative(1000000, 6).toNumber();

const defaultOracleConfig = {
  confFilter: 0.1,
  maxStalenessSlots: null,
};
// hoping that dynamic rate parameter adjustment would be enough to tune their rates to the markets needs
const defaultInterestRate = {
  adjustmentFactor: 0.004, // rate parameters are chosen to be the same for all high asset weight tokens,
  util0: 0.7,
  rate0: 0.1,
  util1: 0.85,
  rate1: 0.2,
  maxRate: 2.0,
};

async function buildAdminClient(): Promise<[MangoClient, Keypair, Keypair]> {
  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER3_KEYPAIR!, 'utf-8'))),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);

  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );

  const creator = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );

  return [client, admin, creator];
}

async function buildUserClient(
  userKeypair: string,
): Promise<[MangoClient, Group, Keypair]> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(userKeypair, 'utf-8'))),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);

  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  const creator = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );
  console.log(`Creator ${creator.publicKey.toBase58()}`);
  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);
  return [client, group, user];
}

async function createGroup() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  console.log(`Creating Group...`);
  const insuranceMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  await client.groupCreate(GROUP_NUM, true, 2, insuranceMint);
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`...registered group ${group.publicKey}`);
}

async function changeAdmin() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  console.log(`Changing admin...`);
  await client.groupEdit(
    group,
    new PublicKey('DSiGNQaKhFCSZbg4HczqCtPAPb1xV51c9GfbfqcVKTB4'),
    new PublicKey('DSiGNQaKhFCSZbg4HczqCtPAPb1xV51c9GfbfqcVKTB4'),
    new PublicKey('DSiGNQaKhFCSZbg4HczqCtPAPb1xV51c9GfbfqcVKTB4'),
  );
}

async function setDepositLimit() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  console.log(`Setting a deposit limit...`);
  await client.groupEdit(
    group,
    new PublicKey('DSiGNQaKhFCSZbg4HczqCtPAPb1xV51c9GfbfqcVKTB4'),
    new PublicKey('DSiGNQaKhFCSZbg4HczqCtPAPb1xV51c9GfbfqcVKTB4'),
    new PublicKey('DSiGNQaKhFCSZbg4HczqCtPAPb1xV51c9GfbfqcVKTB4'),
    undefined,
    undefined,
    toNative(200, 6),
  );
}

async function registerTokens() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  console.log(`Creating USDC stub oracle...`);
  const usdcMainnetMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  await client.stubOracleCreate(group, usdcMainnetMint, 1.0);
  const usdcMainnetOracle = (
    await client.getStubOracle(group, usdcMainnetMint)
  )[0];
  console.log(`...created stub oracle ${usdcMainnetOracle.publicKey}`);

  console.log(`Registering USDC...`);
  await client.tokenRegister(
    group,
    usdcMainnetMint,
    usdcMainnetOracle.publicKey,
    0,
    'USDC',
    {
      ...DefaultTokenRegisterParams,
      initAssetWeight: 1,
      maintAssetWeight: 1,
      initLiabWeight: 1,
      maintLiabWeight: 1,
      liquidationFee: 0,
      netBorrowLimitPerWindowQuote: NET_BORROW_LIMIT_PER_WINDOW_QUOTE,
    },
  );

  console.log(`Registering USDT...`);
  const usdtMainnetMint = new PublicKey(MAINNET_MINTS.get('USDT')!);
  const usdtMainnetOracle = new PublicKey(MAINNET_ORACLES.get('USDT')!);
  await client.tokenRegister(
    group,
    usdtMainnetMint,
    usdtMainnetOracle,
    1,
    'USDT',
    {
      ...DefaultTokenRegisterParams,
      maintAssetWeight: 0.95,
      initAssetWeight: 0.9,
      maintLiabWeight: 1.05,
      initLiabWeight: 1.1,
      liquidationFee: 0.025,
      netBorrowLimitPerWindowQuote: NET_BORROW_LIMIT_PER_WINDOW_QUOTE,
    },
  );

  console.log(`Registering DAI...`);
  const daiMainnetMint = new PublicKey(MAINNET_MINTS.get('DAI')!);
  const daiMainnetOracle = new PublicKey(MAINNET_ORACLES.get('DAI')!);
  await client.tokenRegister(
    group,
    daiMainnetMint,
    daiMainnetOracle,
    2,
    'DAI',
    {
      ...DefaultTokenRegisterParams,
      maintAssetWeight: 0.95,
      initAssetWeight: 0.9,
      maintLiabWeight: 1.05,
      initLiabWeight: 1.1,
      liquidationFee: 0.025,
      netBorrowLimitPerWindowQuote: NET_BORROW_LIMIT_PER_WINDOW_QUOTE,
    },
  );

  console.log(`Registering ETH...`);
  const ethMainnetMint = new PublicKey(MAINNET_MINTS.get('ETH')!);
  const ethMainnetOracle = new PublicKey(MAINNET_ORACLES.get('ETH')!);
  await client.tokenRegister(
    group,
    ethMainnetMint,
    ethMainnetOracle,
    3,
    'ETH',
    {
      ...DefaultTokenRegisterParams,
      maintAssetWeight: 0.9,
      initAssetWeight: 0.8,
      maintLiabWeight: 1.1,
      initLiabWeight: 1.2,
      liquidationFee: 0.05,
      netBorrowLimitPerWindowQuote: NET_BORROW_LIMIT_PER_WINDOW_QUOTE,
    },
  );

  console.log(`Registering SOL...`);
  const solMainnetMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
  const solMainnetOracle = new PublicKey(MAINNET_ORACLES.get('SOL')!);
  await client.tokenRegister(
    group,
    solMainnetMint,
    solMainnetOracle,
    4,
    'SOL',
    {
      ...DefaultTokenRegisterParams,
      maintAssetWeight: 0.9,
      initAssetWeight: 0.8,
      maintLiabWeight: 1.1,
      initLiabWeight: 1.2,
      liquidationFee: 0.05,
      netBorrowLimitPerWindowQuote: NET_BORROW_LIMIT_PER_WINDOW_QUOTE,
    },
  );

  console.log(`Registering MSOL...`);
  const msolMainnetMint = new PublicKey(MAINNET_MINTS.get('MSOL')!);
  const msolMainnetOracle = new PublicKey(MAINNET_ORACLES.get('MSOL')!);
  await client.tokenRegister(
    group,
    msolMainnetMint,
    msolMainnetOracle,
    5,
    'MSOL',
    {
      ...DefaultTokenRegisterParams,
      maintAssetWeight: 0.9,
      initAssetWeight: 0.8,
      maintLiabWeight: 1.1,
      initLiabWeight: 1.2,
      liquidationFee: 0.05,
      netBorrowLimitPerWindowQuote: NET_BORROW_LIMIT_PER_WINDOW_QUOTE,
    },
  );

  console.log(`Registering MNGO...`);
  await client.groupEdit(group, group.admin, group.admin);
  const mngoMainnetMint = new PublicKey(MAINNET_MINTS.get('MNGO')!);
  const mngoMainnetOracle = new PublicKey(MAINNET_ORACLES.get('MNGO')!);
  await client.tokenRegisterTrustless(
    group,
    mngoMainnetMint,
    mngoMainnetOracle,
    6,
    'MNGO',
  );

  console.log(`Registering BONK...`);
  const bonkMainnetMint = new PublicKey(MAINNET_MINTS.get('BONK')!);
  const bonkMainnetOracle = new PublicKey(MAINNET_ORACLES.get('BONK')!);
  await client.tokenRegisterTrustless(
    group,
    bonkMainnetMint,
    bonkMainnetOracle,
    7,
    'BONK',
  );

  // log tokens/banks
  await group.reloadAll(client);
  for (const bank of await Array.from(group.banksMapByMint.values())
    .flat()
    .sort((a, b) => a.tokenIndex - b.tokenIndex)) {
    console.log(`${bank.toString()}`);
  }
}

async function registerSerum3Markets() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  // Register SOL serum market
  await client.serum3RegisterMarket(
    group,
    new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('SOL')!)),
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('USDC')!)),
    0,
    'SOL/USDC',
    0.5,
  );
}

async function createUser(userKeypair: string) {
  const result = await buildUserClient(userKeypair);
  const client = result[0];
  const group = result[1];
  const user = result[2];

  console.log(`Creating MangoAccount...`);
  const mangoAccount = await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  );
  if (!mangoAccount) {
    throw new Error(`MangoAccount not found for user ${user.publicKey}`);
  }

  console.log(`...created MangoAccount ${mangoAccount.publicKey.toBase58()}`);
}

async function depositForUser(userKeypair: string) {
  const result = await buildUserClient(userKeypair);
  const client = result[0];
  const group = result[1];
  const user = result[2];

  const mangoAccount = await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  )!;

  await client.tokenDeposit(
    group,
    mangoAccount!,
    new PublicKey(MAINNET_MINTS.get('USDC')!),
    10,
  );
  await mangoAccount!.reload(client);
  console.log(`...deposited 10 USDC`);
}

async function registerPerpMarkets() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  await client.perpCreateMarket(
    group,
    new PublicKey(MAINNET_ORACLES.get('BTC')!),
    0,
    'BTC-PERP',
    defaultOracleConfig,
    6,
    10, // 0.1$ is the min tick
    100, // if btc price is 20k, one base lot would be 2$
    0.975,
    0.95,
    1.025,
    1.05,
    0.95,
    0.9,
    0.0125,
    -0.0001,
    0.0004,
    5, // note: quote native
    -0.05,
    0.05,
    100, // if btc is at 20k, this is 200$
    true,
    1000, // solana tx fee is currently 50 native quote at a sol price of 10$
    1000000,
    0.01, // less than liquidationFee
    0,
    1.0,
    2 * 60 * 60,
    0.025,
  );

  await client.perpCreateMarket(
    group,
    new PublicKey(MAINNET_ORACLES.get('MNGO')!),
    1,
    'MNGO-PERP-OLD',
    defaultOracleConfig,
    6,
    100, // 0.0001$ is the min tick
    1000000, // if mngo price is 1 cent, one base lot would be 1 cent
    0.995,
    0.99, // 100x leverage
    1.005,
    1.01,
    0,
    0,
    0.0025,
    -0.0001,
    0.0004,
    5,
    -0.05,
    0.05,
    1000, // if mngo price 1 cent, this is 10$
    false,
    1000,
    1000000,
    0.001, // less than liquidationFee
    0,
    1.0,
    2 * 60 * 60,
    0.2, // 20% positive pnl liquidation fee?
  );
}

async function changeTokenOracle() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);
  const bank = group.getFirstBankByMint(
    new PublicKey(MAINNET_MINTS.get('MNGO')!),
  );
  await client.tokenEdit(
    group,
    bank.mint,
    Builder(NullTokenEditParams)
      .oracle(new PublicKey(MAINNET_ORACLES.get('MNGO')!))
      .build(),
  );
}

async function makeTokenReduceonly() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);
  const bank = group.getFirstBankByMint(
    new PublicKey(MAINNET_MINTS.get('DAI')!),
  );
  await client.tokenEdit(
    group,
    bank.mint,
    Builder(NullTokenEditParams).reduceOnly(1).build(),
  );
}

async function changeMaxStalenessSlots() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  for (const bank of Array.from(group.banksMapByTokenIndex.values()).flat()) {
    await client.tokenEdit(
      group,
      bank.mint,
      Builder(NullTokenEditParams)
        .oracleConfig({
          confFilter: 0.1,
          maxStalenessSlots: 120,
        })
        .build(),
    );
  }

  for (const perpMarket of Array.from(
    group.perpMarketsMapByMarketIndex.values(),
  )) {
    await client.perpEditMarket(
      group,
      perpMarket.perpMarketIndex,
      Builder(NullPerpEditParams)
        .oracleConfig({
          confFilter: 0.1,
          maxStalenessSlots: 120,
        })
        .build(),
    );
  }
}

async function changeStartQuote() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  await client.tokenEdit(
    group,
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('USDT')!)).mint,
    Builder(NullTokenEditParams)
      .depositWeightScaleStartQuote(toNative(1000000, 6).toNumber())
      .borrowWeightScaleStartQuote(toNative(1000000, 6).toNumber())
      .build(),
  );
  await client.tokenEdit(
    group,
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('ETH')!)).mint,
    Builder(NullTokenEditParams)
      .depositWeightScaleStartQuote(toNative(100000, 6).toNumber())
      .borrowWeightScaleStartQuote(toNative(100000, 6).toNumber())
      .build(),
  );
  await client.tokenEdit(
    group,
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('SOL')!)).mint,
    Builder(NullTokenEditParams)
      .depositWeightScaleStartQuote(toNative(5000000, 6).toNumber())
      .borrowWeightScaleStartQuote(toNative(5000000, 6).toNumber())
      .build(),
  );
  await client.tokenEdit(
    group,
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('MSOL')!)).mint,
    Builder(NullTokenEditParams)
      .depositWeightScaleStartQuote(toNative(1000000, 6).toNumber())
      .borrowWeightScaleStartQuote(toNative(1000000, 6).toNumber())
      .build(),
  );
  await client.tokenEdit(
    group,
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('MNGO')!)).mint,
    Builder(NullTokenEditParams)
      .depositWeightScaleStartQuote(toNative(5000, 6).toNumber())
      .borrowWeightScaleStartQuote(toNative(5000, 6).toNumber())
      .build(),
  );
  await client.tokenEdit(
    group,
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('BONK')!)).mint,
    Builder(NullTokenEditParams)
      .depositWeightScaleStartQuote(toNative(100000, 6).toNumber())
      .borrowWeightScaleStartQuote(toNative(100000, 6).toNumber())
      .build(),
  );
}

async function makePerpMarketReduceOnly() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];
  const creator = result[2];

  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);
  const perpMarket = group.getPerpMarketByName('MNGO-PERP-OLD');
  await client.perpEditMarket(
    group,
    perpMarket.perpMarketIndex,
    Builder(NullPerpEditParams).reduceOnly(true).build(),
  );
}

async function createAndPopulateAlt() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const creator = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );
  console.log(`Creator ${creator.publicKey.toBase58()}`);
  const group = await client.getGroupForCreator(creator.publicKey, GROUP_NUM);

  const connection = client.program.provider.connection;

  // Create ALT, and set to group at index 0
  if (group.addressLookupTables[0].equals(PublicKey.default)) {
    try {
      console.log(`ALT: Creating`);
      const createIx = AddressLookupTableProgram.createLookupTable({
        authority: admin.publicKey,
        payer: admin.publicKey,
        recentSlot: await connection.getSlot('finalized'),
      });
      const createTx = await buildVersionedTx(
        client.program.provider as AnchorProvider,
        [createIx[0]],
      );
      let sig = await connection.sendTransaction(createTx);
      console.log(
        `...created ALT ${createIx[1]} https://explorer.solana.com/tx/${sig}`,
      );

      console.log(`ALT: set at index 0 for group...`);
      sig = (await client.altSet(group, createIx[1], 0)).signature;
      console.log(`...https://explorer.solana.com/tx/${sig}`);
    } catch (error) {
      console.log(error);
    }
  }

  // Extend using mango v4 relevant pub keys
  try {
    const bankAddresses = Array.from(group.banksMapByMint.values())
      .flat()
      .map((bank) => [bank.publicKey, bank.oracle, bank.vault])
      .flat()
      .concat(
        Array.from(group.banksMapByMint.values())
          .flat()
          .map((mintInfo) => mintInfo.publicKey),
      );

    const serum3MarketAddresses = Array.from(
      group.serum3MarketsMapByExternal.values(),
    )
      .flat()
      .map((serum3Market) => serum3Market.publicKey);

    const serum3ExternalMarketAddresses = Array.from(
      group.serum3ExternalMarketsMap.values(),
    )
      .flat()
      .map((serum3ExternalMarket) => [
        serum3ExternalMarket.publicKey,
        serum3ExternalMarket.bidsAddress,
        serum3ExternalMarket.asksAddress,
      ])
      .flat();

    const perpMarketAddresses = Array.from(
      group.perpMarketsMapByMarketIndex.values(),
    )
      .flat()
      .map((perpMarket) => [
        perpMarket.publicKey,
        perpMarket.oracle,
        perpMarket.bids,
        perpMarket.asks,
        perpMarket.eventQueue,
      ])
      .flat();

    // eslint-disable-next-line no-inner-declarations
    async function extendTable(addresses: PublicKey[]): Promise<void> {
      await group.reloadAll(client);
      const alt =
        await client.program.provider.connection.getAddressLookupTable(
          group.addressLookupTables[0],
        );

      addresses = addresses.filter(
        (newAddress) =>
          alt.value?.state.addresses &&
          alt.value?.state.addresses.findIndex((addressInALt) =>
            addressInALt.equals(newAddress),
          ) === -1,
      );
      if (addresses.length === 0) {
        return;
      }
      const extendIx = AddressLookupTableProgram.extendLookupTable({
        lookupTable: group.addressLookupTables[0],
        payer: admin.publicKey,
        authority: admin.publicKey,
        addresses,
      });
      const extendTx = await buildVersionedTx(
        client.program.provider as AnchorProvider,
        [extendIx],
      );
      const sig = await client.program.provider.connection.sendTransaction(
        extendTx,
      );
      console.log(`https://explorer.solana.com/tx/${sig}`);
    }

    console.log(`ALT: extending using mango v4 relevant public keys`);

    await extendTable(bankAddresses);
    await extendTable([OPENBOOK_PROGRAM_ID['mainnet-beta']]);
    await extendTable(serum3MarketAddresses);
    await extendTable(serum3ExternalMarketAddresses);

    // TODO: dont extend for perps atm
    await extendTable(perpMarketAddresses);

    // Well known addresses
    await extendTable([
      SystemProgram.programId,
      SYSVAR_RENT_PUBKEY,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      NATIVE_MINT,
      SYSVAR_INSTRUCTIONS_PUBKEY,
      ComputeBudgetProgram.programId,
    ]);
  } catch (error) {
    console.log(error);
  }
}

async function main() {
  try {
    // await createGroup();
    // await changeAdmin();
    // await setDepositLimit();
  } catch (error) {
    console.log(error);
  }
  try {
    // await registerTokens();
    // await changeTokenOracle();
    // await makeTokenReduceonly();
    // await changeMaxStalenessSlots();
    // await changeStartQuote();
  } catch (error) {
    console.log(error);
  }
  try {
    // await registerSerum3Markets();
  } catch (error) {
    console.log(error);
  }

  try {
    // await registerPerpMarkets();
    // await makePerpMarketReduceOnly();
  } catch (error) {
    console.log(error);
  }
  try {
    // await createUser(MB_USER_KEYPAIR!);
    // depositForUser(MB_USER_KEYPAIR!);
  } catch (error) {
    console.log(error);
  }

  try {
    createAndPopulateAlt();
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}

////////////////////////////////////////////////////////////
/// UNUSED /////////////////////////////////////////////////
////////////////////////////////////////////////////////////

async function expandMangoAccount(userKeypair: string) {
  const result = await buildUserClient(userKeypair);
  const client = result[0];
  const group = result[1];
  const user = result[2];

  const mangoAccounts = await client.getMangoAccountsForOwner(
    group,
    user.publicKey,
  );
  if (!mangoAccounts) {
    throw new Error(`MangoAccounts not found for user ${user.publicKey}`);
  }

  for (const mangoAccount of mangoAccounts) {
    console.log(
      `...expanding MangoAccount ${mangoAccount.publicKey.toBase58()}`,
    );
    await client.expandMangoAccount(group, mangoAccount, 8, 8, 8, 8);
  }
}

async function placeSerum3TradeAndCancelIt(userKeypair: string) {
  const result = await buildUserClient(userKeypair);
  const client = result[0];
  const group = result[1];
  const user = result[2];

  const mangoAccounts = await client.getMangoAccountsForOwner(
    group,
    user.publicKey,
  );
  if (!mangoAccounts) {
    throw new Error(`MangoAccounts not found for user ${user.publicKey}`);
  }

  for (const mangoAccount of mangoAccounts) {
    console.log(`...found MangoAccount ${mangoAccount.publicKey.toBase58()}`);
    console.log(`...placing serum3 order`);
    await client.serum3PlaceOrder(
      group,
      mangoAccount,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
      Serum3Side.bid,
      1,
      1,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );
    console.log(`...current own orders on OB`);
    let orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
      client,
      group,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
    );
    for (const order of orders) {
      console.log(order);
    }
    console.log(`...cancelling serum3 orders`);
    await client.serum3CancelAllOrders(
      group,
      mangoAccount,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
      10,
    );
    console.log(`...current own orders on OB`);
    orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
      client,
      group,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
    );
    for (const order of orders) {
      console.log(order);
    }
  }
}

async function deregisterSerum3Markets() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  // change xxx/xxx to market of choice
  const serum3Market = group.getSerum3MarketByName('XXX/XXX');
  const sig = await client.serum3deregisterMarket(
    group,
    serum3Market.serumMarketExternal,
  );
  console.log(
    `...deregistered serum market ${serum3Market.name}, sig https://explorer.solana.com/tx/${sig}`,
  );
}

async function deregisterTokens() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  // change -1 to tokenIndex of choice
  const bank = group.getFirstBankByTokenIndex(-1 as TokenIndex);
  const sig = await client.tokenDeregister(group, bank.mint);
  console.log(
    `...removed token ${bank.name}, sig https://explorer.solana.com/tx/${sig}`,
  );
}
