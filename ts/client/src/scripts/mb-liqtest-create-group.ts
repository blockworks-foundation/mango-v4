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
const GROUP_NUM = Number(process.env.GROUP_NUM || 200);

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['MNGO', 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac'],
]);

const STUB_PRICES = new Map([
  ['USDC', 1.0],
  ['BTC', 20000.0], // btc and usdc both have 6 decimals
  ['SOL', 0.04], // sol has 9 decimals, equivalent to $40 per SOL
  ['MNGO', 0.04], // same price/decimals as SOL for convenience
]);

// External markets are matched with those in https://github.com/blockworks-foundation/mango-client-v3/blob/main/src/ids.json
// and verified to have best liquidity for pair on https://openserum.io/
const MAINNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', 'A8YFbxQYFVqKZaoYJLLUVcQiWP7G2MeEgW5wsAQgMvFw'],
  ['SOL/USDC', '9wFFyRfZBsuAha4YcuxcXLKwMxJR43S7fPfQLusDBzvT'],
]);

const MIN_VAULT_TO_DEPOSITS_RATIO = 0.2;
const NET_BORROWS_WINDOW_SIZE_TS = 24 * 60 * 60;
const NET_BORROWS_LIMIT_NATIVE = 1 * Math.pow(10, 7) * Math.pow(10, 6);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL!, options);

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
    {
      idsSource: 'get-program-accounts',
    },
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
      const price = STUB_PRICES.get(name)!;
      await client.stubOracleCreate(group, mintPk, price);
    } catch (error) {
      console.log(error);
    }
    const oracle = (await client.getStubOracle(group, mintPk))[0];
    console.log(`...created stub oracle ${oracle.publicKey}`);
    oracles.set(name, oracle.publicKey);
  }

  const defaultOracleConfig = {
    confFilter: 0.1,
    maxStalenessSlots: null,
  };
  const defaultInterestRate = {
    adjustmentFactor: 0.01,
    util0: 0.4,
    rate0: 0.07,
    util1: 0.8,
    rate1: 0.9,
    maxRate: 1.5,
  };

  // register token 0
  console.log(`Registering USDC...`);
  const usdcMainnetMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  const usdcMainnetOracle = oracles.get('USDC');
  try {
    await client.tokenRegister(
      group,
      usdcMainnetMint,
      usdcMainnetOracle,
      defaultOracleConfig,
      0,
      'USDC',
      defaultInterestRate,
      0.0,
      0.0001,
      1,
      1,
      1,
      1,
      0,
      MIN_VAULT_TO_DEPOSITS_RATIO,
      NET_BORROWS_WINDOW_SIZE_TS,
      NET_BORROWS_LIMIT_NATIVE,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
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
      defaultOracleConfig,
      1,
      'BTC',
      defaultInterestRate,
      0.0,
      0.0001,
      0.9,
      0.8,
      1.1,
      1.2,
      0.05,
      MIN_VAULT_TO_DEPOSITS_RATIO,
      NET_BORROWS_WINDOW_SIZE_TS,
      NET_BORROWS_LIMIT_NATIVE,
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
      defaultOracleConfig,
      2, // tokenIndex
      'SOL',
      defaultInterestRate,
      0.0,
      0.0001,
      0.9,
      0.8,
      1.1,
      1.2,
      0.05,
      MIN_VAULT_TO_DEPOSITS_RATIO,
      NET_BORROWS_WINDOW_SIZE_TS,
      NET_BORROWS_LIMIT_NATIVE,
    );
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // log tokens/banks
  for (const bank of await group.banksMapByMint.values()) {
    console.log(`${bank.toString()}`);
  }

  console.log('Registering SOL/USDC serum market...');
  try {
    await client.serum3RegisterMarket(
      group,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
      group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('SOL')!)),
      group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('USDC')!)),
      1,
      'SOL/USDC',
    );
  } catch (error) {
    console.log(error);
  }

  console.log('Registering MNGO-PERP market...');
  const mngoMainnetOracle = oracles.get('MNGO');
  try {
    await client.perpCreateMarket(
      group,
      mngoMainnetOracle,
      0,
      'MNGO-PERP',
      defaultOracleConfig,
      9,
      10,
      100000, // base lots
      0.9,
      0.8,
      1.1,
      1.2,
      0.05,
      -0.001,
      0.002,
      0,
      -0.1,
      0.1,
      10,
      false,
      false,
      0,
      0,
      0,
      0,
      1.0,
      2 * 60 * 60,
    );
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

main();
