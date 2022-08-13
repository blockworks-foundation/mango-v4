import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// An example for admins based on high level api i.e. the client
// Depoys a new mango group to devnet, registers 4 tokens, and 1 serum3 spot market
//
// process.env.ADMIN_KEYPAIR - group admin keypair path
// to create a new admin keypair:
// * solana-keygen new --outfile ~/.config/solana/admin.json
// * solana airdrop 1  -k ~/.config/solana/admin.json
//

const DEVNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', 'DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB'],
  ['SOL/USDC', '5xWpt56U1NCuHoAEtpLeUrQcxDkEpNfScjfLFaRzLPgR'],
]);
const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['ORCA', 'orcarKHSqC5CDDsGbho8GKvwExejWHxTqGzXgcewB9L'],
  ['MNGO', 'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC'],
]);
const DEVNET_ORACLES = new Map([
  ['BTC', 'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J'],
  ['SOL', 'J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix'],
  ['ORCA', 'A1WttWF7X3Rg6ZRpB2YQUFHCRh1kiXV8sKKLV3S9neJV'],
  ['MNGO', '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o'],
]);

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );

  // group
  console.log(`Creating Group...`);
  const insuranceMint = new PublicKey(DEVNET_MINTS.get('USDC')!);
  try {
    await client.groupCreate(GROUP_NUM, true, 0, insuranceMint);
  } catch (error) {
    console.log(error);
  }
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`...registered group ${group.publicKey}`);

  // stub oracle + register token 0
  console.log(`Registering USDC...`);
  const usdcDevnetMint = new PublicKey(DEVNET_MINTS.get('USDC')!);
  try {
    await client.stubOracleCreate(group, usdcDevnetMint, 1.0);
  } catch (error) {
    console.log(error);
  }
  const usdcDevnetOracle = (
    await client.getStubOracle(group, usdcDevnetMint)
  )[0];
  console.log(`...created stub oracle ${usdcDevnetOracle.publicKey}`);
  try {
    await client.tokenRegister(
      group,
      usdcDevnetMint,
      usdcDevnetOracle.publicKey,
      0.1,
      0, // tokenIndex
      'USDC',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      1.5,
      0.0005,
      0.0005,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reloadAll(client);
  } catch (error) {}

  // register token 1
  console.log(`Registering BTC...`);
  const btcDevnetMint = new PublicKey(DEVNET_MINTS.get('BTC')!);
  const btcDevnetOracle = new PublicKey(DEVNET_ORACLES.get('BTC')!);
  try {
    await client.tokenRegister(
      group,
      btcDevnetMint,
      btcDevnetOracle,
      0.1,
      1, // tokenIndex
      'BTC',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      0.88,
      0.0005,
      0.0005,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // register token 2
  console.log(`Registering SOL...`);
  const solDevnetMint = new PublicKey(DEVNET_MINTS.get('SOL')!);
  const solDevnetOracle = new PublicKey(DEVNET_ORACLES.get('SOL')!);
  try {
    await client.tokenRegister(
      group,
      solDevnetMint,
      solDevnetOracle,
      0.1,
      2, // tokenIndex
      'SOL',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      0.63,
      0.0005,
      0.0005,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // register token 3
  console.log(`Registering ORCA...`);
  const orcaDevnetMint = new PublicKey(DEVNET_MINTS.get('ORCA')!);
  const orcaDevnetOracle = new PublicKey(DEVNET_ORACLES.get('ORCA')!);
  try {
    await client.tokenRegister(
      group,
      orcaDevnetMint,
      orcaDevnetOracle,
      0.1,
      3, // tokenIndex
      'ORCA',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      0.63,
      0.0005,
      0.0005,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // register token 4
  console.log(
    `Editing group, setting existing admin as fastListingAdmin to be able to add MNGO truslessly...`,
  );
  let sig = await client.groupEdit(
    group,
    group.admin,
    new PublicKey('Efhak3qj3MiyzgJr3cUUqXXz5wr3oYHt9sPzuqJf9eBN'),
    undefined,
    undefined,
  );
  console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
  console.log(`Registering MNGO...`);
  const mngoDevnetMint = new PublicKey(DEVNET_MINTS.get('MNGO')!);
  const mngoDevnetOracle = new PublicKey(DEVNET_ORACLES.get('MNGO')!);
  try {
    await client.tokenRegisterTrustless(
      group,
      mngoDevnetMint,
      mngoDevnetOracle,
      4,
      'MNGO',
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // log tokens/banks
  for (const bank of await group.banksMap.values()) {
    console.log(
      `...registered Bank ${bank.tokenIndex} ${bank.publicKey}, mint ${bank.mint}, oracle ${bank.oracle}`,
    );
    console.log(bank.toString());
  }

  // register serum market
  console.log(`Registering serum3 market...`);
  const serumMarketExternalPk = new PublicKey(
    DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  );
  try {
    await client.serum3RegisterMarket(
      group,

      serumMarketExternalPk,
      group.banksMap.get('BTC')!,
      group.banksMap.get('USDC')!,
      0,
      'BTC/USDC',
    );
  } catch (error) {
    console.log(error);
  }
  const markets = await client.serum3GetMarkets(
    group,
    group.banksMap.get('BTC')?.tokenIndex,
    group.banksMap.get('USDC')?.tokenIndex,
  );
  console.log(`...registerd serum3 market ${markets[0].publicKey}`);

  // register perp market
  console.log(`Registering perp market...`);
  try {
    await client.perpCreateMarket(
      group,
      btcDevnetOracle,
      0,
      'BTC-PERP',
      0.1,
      1,
      6,
      1,
      10,
      100,
      0.975,
      0.95,
      1.025,
      1.05,
      0.012,
      0.0002,
      0.0,
      0.05,
      0.05,
      100,
    );
    console.log('done');
  } catch (error) {
    console.log(error);
  }
  const perpMarkets = await client.perpGetMarkets(
    group,
    group.banksMap.get('BTC')?.tokenIndex,
  );
  console.log(`...created perp market ${perpMarkets[0].publicKey}`);

  //
  // edit
  //

  console.log(`Editing USDC...`);
  try {
    let sig = await client.tokenEdit(
      group,
      'USDC',
      btcDevnetOracle,
      0.1,
      undefined,
      0.01,
      0.3,
      0.08,
      0.81,
      0.91,
      0.75,
      0.0007,
      1.7,
      0.9,
      0.7,
      1.3,
      1.5,
      0.04,
    );
    console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    await group.reloadAll(client);
    console.log(group.banksMap.get('USDC')!.toString());
  } catch (error) {
    throw error;
  }
  console.log(`Resetting USDC...`);
  try {
    let sig = await client.tokenEdit(
      group,
      'USDC',
      usdcDevnetOracle.publicKey,
      0.1,
      undefined,
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      1.5,
      0.0005,
      0.0005,
      1.0,
      1.0,
      1.0,
      1.0,
      0.02,
    );
    console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    await group.reloadAll(client);
    console.log(group.banksMap.get('USDC').toString());
  } catch (error) {
    throw error;
  }

  console.log(`Editing perp market...`);
  try {
    let sig = await client.perpEditMarket(
      group,
      'BTC-PERP',
      btcDevnetOracle,
      0.2,
      0,
      6,
      0.9,
      0.9,
      1.035,
      1.06,
      0.013,
      0.0003,
      0.1,
      0.07,
      0.07,
      1001,
    );
    console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    await group.reloadAll(client);
    console.log(group.perpMarketsMap.get('BTC-PERP').toString());
  } catch (error) {
    console.log(error);
  }
  console.log(`Resetting perp market...`);
  try {
    let sig = await client.perpEditMarket(
      group,
      'BTC-PERP',
      btcDevnetOracle,
      0.1,
      1,
      6,
      1,
      0.95,
      1.025,
      1.05,
      0.012,
      0.0002,
      0.0,
      0.05,
      0.05,
      100,
    );
    console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    await group.reloadAll(client);
    console.log(group.perpMarketsMap.get('BTC-PERP').toString());
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

main();
