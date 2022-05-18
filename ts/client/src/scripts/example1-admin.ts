import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { DEVNET_SERUM3_PROGRAM_ID } from '../constants';

const DEVNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', 'DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB'],
]);
const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'],
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
]);
const DEVNET_ORACLES = new Map([
  ['BTC', 'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J'],
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
  const client = await MangoClient.connect(adminProvider, true);

  // group
  console.log(`Creating Group...`);
  try {
    await client.createGroup();
  } catch (error) {}
  const group = await client.getGroupForAdmin(admin.publicKey);
  console.log(`...registered group ${group.publicKey}`);

  // register token 0
  console.log(`Registering BTC...`);
  const btcDevnetMint = new PublicKey(DEVNET_MINTS.get('BTC')!);
  const btcDevnetOracle = new PublicKey(DEVNET_ORACLES.get('BTC')!);
  try {
    await client.registerToken(
      group,
      btcDevnetMint,
      btcDevnetOracle,
      0,
      'BTC',
      0.4,
      0.07,
      0.8,
      0.9,
      0.0005,
      0.0005,
      1.5,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reload(client);
  } catch (error) {}

  // stub oracle + register token 1
  console.log(`Registering USDC...`);
  const usdcDevnetMint = new PublicKey(DEVNET_MINTS.get('USDC')!);
  try {
    await client.createStubOracle(group, usdcDevnetMint, 1.0);
  } catch (error) {}
  const usdcDevnetOracle = await client.getStubOracle(group, usdcDevnetMint);
  try {
    await client.registerToken(
      group,
      usdcDevnetMint,
      usdcDevnetOracle.publicKey,
      1,
      'USDC',
      0.4,
      0.07,
      0.8,
      0.9,
      0.0005,
      0.0005,
      1.5,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reload(client);
  } catch (error) {}

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
      DEVNET_SERUM3_PROGRAM_ID,
      serumMarketExternalPk,
      group.banksMap.get('BTC')!,
      group.banksMap.get('USDC')!,
      0,
      'BTC/USDC',
    );
  } catch (error) {}
  const markets = await client.serum3GetMarket(
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
      0,
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
  } catch (error) {
    console.log(error);
  }
  const perpMarkets = await client.perpGetMarket(
    group,
    group.banksMap.get('BTC')?.tokenIndex,
    group.banksMap.get('USDC')?.tokenIndex,
  );
  console.log(`...created perp market ${perpMarkets[0].publicKey}`);

  process.exit();
}

main();
