import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['USDT', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'], // Wrapped Bitcoin (Sollet)
  ['ETH', '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'], // Ether (Portal)
  ['soETH', '2FPyTwcZLUg1MDrwsyoP4D6s1tM7hAkHYRjkNb5w6Pxk'], // Wrapped Ethereum (Sollet)
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['MSOL', 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So'],
  ['MNGO', 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac'],
]);
const MAINNET_ORACLES = new Map([
  ['USDT', '3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL'],
  ['BTC', 'GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'],
  ['ETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['soETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['SOL', 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'],
  ['MSOL', 'E4v1BBgoso9s64TQvmyownAVJbhbEPGyzA3qn4n46qj9'],
  ['MNGO', '79wm3jjcPr6RaNQ4DGvP5KxG1mNd3gEBsg6FsNVFezK4'],
]);

async function createGroup() {
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL, options);

  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  console.log(`Creating Group...`);
  const insuranceMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  await client.groupCreate(2, true, 0, insuranceMint);
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`...registered group ${group.publicKey}`);
}

async function registerTokens() {
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL, options);

  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  const group = await client.getGroupForCreator(admin.publicKey, 2);

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
    0.1,
    0,
    'USDC',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
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
    0.1,
    1,
    'USDT',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.95,
    0.9,
    1.05,
    1.1,
    0.025,
  );

  console.log(`Registering BTC...`);
  const btcMainnetMint = new PublicKey(MAINNET_MINTS.get('BTC')!);
  const btcMainnetOracle = new PublicKey(MAINNET_ORACLES.get('BTC')!);
  await client.tokenRegister(
    group,
    btcMainnetMint,
    btcMainnetOracle,
    0.1,
    2,
    'BTC',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
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
    0.1,
    3,
    'ETH',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
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
    0.1,
    4,
    'soETH',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
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
    0.1,
    5,
    'SOL',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering MSOL...`);
  const msolMainnetMint = new PublicKey(MAINNET_MINTS.get('MSOL')!);
  const msolMainnetOracle = new PublicKey(MAINNET_ORACLES.get('MSOL')!);
  await client.tokenRegister(
    group,
    msolMainnetMint,
    msolMainnetOracle,
    0.1,
    6,
    'MSOL',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
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
  for (const bank of await group.banksMap.values()) {
    console.log(`${bank.toString()}`);
  }
}

async function createUser() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);

  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`${group.toString()}`);

  console.log(`Creating MangoAccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  );
  console.log(`...created MangoAccount ${mangoAccount.publicKey.toBase58()}`);
  console.log(mangoAccount.toString(group));

  await client.tokenDeposit(group, mangoAccount, 'USDC', 10);
  await mangoAccount.reload(client, group);
  console.log(`...deposited 10 USDC`);

  await client.tokenDeposit(group, mangoAccount, 'SOL', 1);
  await mangoAccount.reload(client, group);
  console.log(`...deposited 1 SOL`);
}

async function main() {
  await createGroup();
  await registerTokens();
  // createUser();
}

try {
  main();
} catch (error) {
  console.log(error);
}
