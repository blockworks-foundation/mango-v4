import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

const DEVNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', 'DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB'],
  ['SOL/USDC', '5xWpt56U1NCuHoAEtpLeUrQcxDkEpNfScjfLFaRzLPgR'],
]);
const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['ORCA', 'orcarKHSqC5CDDsGbho8GKvwExejWHxTqGzXgcewB9L'],
]);
const DEVNET_ORACLES = new Map([
  ['BTC', 'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J'],
  ['SOL', 'J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix'],
  ['ORCA', 'A1WttWF7X3Rg6ZRpB2YQUFHCRh1kiXV8sKKLV3S9neJV'],
]);

//
// An example for admins based on high level api i.e. the client
// Depoys a new mango group to devnet, registers 2 tokens, and 1 serum3 spot market
//
// process.env.ADMIN_KEYPAIR - group admin keypair path
// to create a new admin keypair:
// * solana-keygen new --outfile ~/.config/solana/admin.json
// * solana airdrop 1  -k ~/.config/solana/admin.json
//
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
  try {
    await client.createGroup(0, true);
  } catch (error) {
    console.log(error);
  }
  const group = await client.getGroupForAdmin(admin.publicKey);
  console.log(`...registered group ${group.publicKey}`);

  // register token 0
  console.log(`Registering BTC...`);
  const btcDevnetMint = new PublicKey(DEVNET_MINTS.get('BTC')!);
  const btcDevnetOracle = new PublicKey(DEVNET_ORACLES.get('BTC')!);
  try {
    await client.tokenRegister(
      group,
      btcDevnetMint,
      btcDevnetOracle,
      0.1,
      0,
      'BTC',
      0.4,
      0.07,
      0.8,
      0.9,
      0.88,
      0.0005,
      1.5,
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

  // stub oracle + register token 1
  console.log(`Registering USDC...`);
  const usdcDevnetMint = new PublicKey(DEVNET_MINTS.get('USDC')!);
  try {
    await client.createStubOracle(group, usdcDevnetMint, 1.0);
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
      1,
      'USDC',
      0.4,
      0.07,
      0.8,
      0.9,
      1.5,
      0.0005,
      1.5,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reloadAll(client);
  } catch (error) {}

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
      0.4,
      0.07,
      0.8,
      0.9,
      0.63,
      0.0005,
      1.5,
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
      0.4,
      0.07,
      0.8,
      0.9,
      0.63,
      0.0005,
      1.5,
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

  // log tokens/banks
  for (const bank of await group.banksMap.values()) {
    console.log(
      `...registered Bank ${bank.tokenIndex} ${bank.publicKey}, mint ${bank.mint}, oracle ${bank.oracle}`,
    );
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
      'BTC/USDC',
      0.1,
      0,
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
    group.banksMap.get('USDC')?.tokenIndex,
  );
  console.log(`...created perp market ${perpMarkets[0].publicKey}`);

  process.exit();
}

main();
