import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// Script which depoys a new mango group, and registers 3 tokens
// with stub oracles
//

// default to group 1, to not conflict with the normal group
const GROUP_NUM = Number(process.env.GROUP_NUM || 1);

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
]);

const STUB_PRICES = new Map([
  ['USDC', 1.0],
  ['BTC', 20000.0], // btc and usdc both have 6 decimals
  ['SOL', 0.04], // sol has 9 decimals, equivalent to $40 per SOL
]);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  // group
  console.log(`Creating Group...`);
  try {
    const insuranceMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
    await client.groupCreate(GROUP_NUM, true, 0, insuranceMint);
  } catch (error) {
    console.log(error);
  }
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`...registered group ${group.publicKey}`);

  // stub oracles
  let oracles = new Map();
  for (let [name, mint] of MAINNET_MINTS) {
    console.log(`Creating stub oracle for ${name}...`);
    const mintPk = new PublicKey(mint);
    try {
      await client.stubOracleCreate(group, mintPk, STUB_PRICES[name]);
    } catch (error) {
      console.log(error);
    }
    const oracle = (await client.getStubOracle(group, mintPk))[0];
    console.log(`...created stub oracle ${oracle.publicKey}`);
    oracles.set(name, oracle.publicKey);
  }

  // register token 1
  console.log(`Registering BTC...`);
  const btcMainnetMint = new PublicKey(MAINNET_MINTS.get('BTC')!);
  const btcMainnetOracle = oracles.get('BTC');
  try {
    await client.tokenRegister(
      group,
      btcMainnetMint,
      btcMainnetOracle,
      0.1,
      1,
      'BTC',
      0.01,
      0.4,
      0.07,
      0.7,
      0.88,
      1.5,
      0.0005,
      1.5,
      0.9,
      0.8,
      1.1,
      1.2,
      0.05,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // register token 0
  console.log(`Registering USDC...`);
  const usdcMainnetMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  const usdcMainnetOracle = oracles.get('USDC');
  try {
    await client.tokenRegister(
      group,
      usdcMainnetMint,
      usdcMainnetOracle,
      0.1,
      0,
      'USDC',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      1.5,
      0.0005,
      1.5,
      1,
      1,
      1,
      1,
      0,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // register token 2
  console.log(`Registering SOL...`);
  const solMainnetMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
  const solMainnetOracle = oracles.get('SOL');
  try {
    await client.tokenRegister(
      group,
      solMainnetMint,
      solMainnetOracle,
      0.1,
      2, // tokenIndex
      'SOL',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      1.5,
      0.0005,
      1.5,
      0.9,
      0.8,
      1.1,
      1.2,
      0.05,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // log tokens/banks
  for (const bank of await group.banksMap.values()) {
    console.log(`${bank.toString()}`);
  }

  process.exit();
}

main();
