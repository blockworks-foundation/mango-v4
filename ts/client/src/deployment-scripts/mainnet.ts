import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID, MSRM_MINTS } from '../constants';
import { InterestRateParams } from '../types';

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

// Reference
// https://explorer.solana.com/
// https://github.com/blockworks-foundation/mango-client-v3/blob/main/src/ids.json
const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['USDT', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'], // Wrapped Bitcoin (Sollet)
  ['ETH', '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'], // Ether (Portal), will be treat as ETH due to higher liquidity
  ['soETH', '2FPyTwcZLUg1MDrwsyoP4D6s1tM7hAkHYRjkNb5w6Pxk'], // Wrapped Ethereum (Sollet), will be treated as soETH
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['mSOL', 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So'],
  ['MNGO', 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac'],
]);

// Reference
// https://pyth.network/price-feeds/
// https://switchboard.xyz/explorer
const MAINNET_ORACLES = new Map([
  ['USDT', '3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL'],
  ['BTC', 'GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'],
  ['ETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['soETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['SOL', 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'],
  ['mSOL', 'E4v1BBgoso9s64TQvmyownAVJbhbEPGyzA3qn4n46qj9'],
  ['MNGO', '79wm3jjcPr6RaNQ4DGvP5KxG1mNd3gEBsg6FsNVFezK4'],
]);

async function createGroup() {
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

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

  console.log(`Creating Group...`);
  const insuranceMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  await client.groupCreate(
    GROUP_NUM,
    true /* with intention */,
    0 /* since spot and perp features are not finished */,
    insuranceMint,
    MSRM_MINTS['mainnet-beta'],
  );
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`...registered group ${group.publicKey}`);
}

async function registerTokens() {
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    } /* idsjson service doesn't know about this group yet */,
  );

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

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
    defaultOracleConfig,
    0, // insurance vault token should be the first to be registered
    'USDC',
    defaultInterestRate,
    0.005, // 50 bps
    0.0005, // 5 bps
    1,
    1,
    1,
    1,
    0,
  );

  console.log(`Registering USDT...`);
  const usdtMainnetMint = new PublicKey(MAINNET_MINTS.get('USDT')!);
  const usdtMainnetOracle = new PublicKey(MAINNET_ORACLES.get('USDT')!);
  await client.tokenRegister(
    group,
    usdtMainnetMint,
    usdtMainnetOracle,
    defaultOracleConfig,
    1,
    'USDT',
    defaultInterestRate,
    0.005,
    0.0005,
    0.95,
    0.9,
    1.05,
    1.1,
    0.025, // rule of thumb used - half of maintLiabWeight
  );

  console.log(`Registering BTC...`);
  const btcMainnetMint = new PublicKey(MAINNET_MINTS.get('BTC')!);
  const btcMainnetOracle = new PublicKey(MAINNET_ORACLES.get('BTC')!);
  await client.tokenRegister(
    group,
    btcMainnetMint,
    btcMainnetOracle,
    defaultOracleConfig,
    2,
    'BTC',
    defaultInterestRate,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering ETH...`);
  const ethMainnetMint = new PublicKey(MAINNET_MINTS.get('ETH')!);
  const ethMainnetOracle = new PublicKey(MAINNET_ORACLES.get('ETH')!);
  await client.tokenRegister(
    group,
    ethMainnetMint,
    ethMainnetOracle,
    defaultOracleConfig,
    3,
    'ETH',
    defaultInterestRate,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering soETH...`);
  const soEthMainnetMint = new PublicKey(MAINNET_MINTS.get('soETH')!);
  const soEthMainnetOracle = new PublicKey(MAINNET_ORACLES.get('soETH')!);
  await client.tokenRegister(
    group,
    soEthMainnetMint,
    soEthMainnetOracle,
    defaultOracleConfig,
    4,
    'soETH',
    defaultInterestRate,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering SOL...`);
  const solMainnetMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
  const solMainnetOracle = new PublicKey(MAINNET_ORACLES.get('SOL')!);
  await client.tokenRegister(
    group,
    solMainnetMint,
    solMainnetOracle,
    defaultOracleConfig,
    5,
    'SOL',
    defaultInterestRate,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering mSOL...`);
  const msolMainnetMint = new PublicKey(MAINNET_MINTS.get('mSOL')!);
  const msolMainnetOracle = new PublicKey(MAINNET_ORACLES.get('mSOL')!);
  await client.tokenRegister(
    group,
    msolMainnetMint,
    msolMainnetOracle,
    defaultOracleConfig,
    6,
    'mSOL',
    defaultInterestRate,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  // log tokens/banks
  await group.reloadAll(client);
  for (const [bank] of await group.banksMapByMint.values()) {
    console.log(`${bank.toString()}`);
  }
}

async function main() {
  createGroup();
  registerTokens();
}

try {
  main();
} catch (error) {
  console.log(error);
}
