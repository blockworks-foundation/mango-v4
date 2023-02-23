import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  AddressLookupTableProgram,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';

//
// Script which depoys a new mango group, and registers 3 tokens
// with stub oracles
//

// default to group 1, to not conflict with the normal group
const GROUP_NUM = Number(process.env.GROUP_NUM || 200);

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['ETH', '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['MNGO', 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac'],
]);

const STUB_PRICES = new Map([
  ['USDC', 1.0],
  ['ETH', 1200.0], // eth and usdc both have 6 decimals
  ['SOL', 0.015], // sol has 9 decimals, equivalent to $15 per SOL
  ['MNGO', 0.02], // same price/decimals as SOL for convenience
]);

// External markets are matched with those in https://github.com/blockworks-foundation/mango-client-v3/blob/main/src/ids.json
// and verified to have best liquidity for pair on https://openserum.io/
const MAINNET_SERUM3_MARKETS = new Map([
  ['ETH/USDC', 'FZxi3yWkE5mMjyaZj6utmYL54QQYfMCKMcLaQZq4UwnA'],
  ['SOL/USDC', '8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6'],
]);

const MIN_VAULT_TO_DEPOSITS_RATIO = 0.2;
const NET_BORROWS_WINDOW_SIZE_TS = 24 * 60 * 60;
const NET_BORROWS_LIMIT_NATIVE = 1 * Math.pow(10, 7) * Math.pow(10, 6);

async function main() {
  const options = AnchorProvider.defaultOptions();
  options.commitment = 'processed';
  options.preflightCommitment = 'finalized';
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
      prioritizationFee: 100,
      txConfirmationCommitment: 'confirmed',
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
  console.log(`Registering ETH...`);
  const ethMainnetMint = new PublicKey(MAINNET_MINTS.get('ETH')!);
  const ethMainnetOracle = oracles.get('ETH');
  try {
    await client.tokenRegister(
      group,
      ethMainnetMint,
      ethMainnetOracle,
      defaultOracleConfig,
      1,
      'ETH',
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
      6,
      10,
      100000, // base lots
      0.9,
      0.8,
      1.1,
      1.2,
      0.0,
      0.0,
      0.05,
      -0.001,
      0.002,
      0,
      -0.1,
      0.1,
      10,
      false,
      0,
      0,
      0,
      0,
      -1.0,
      2 * 60 * 60,
      0.025,
    );
  } catch (error) {
    console.log(error);
  }

  await createAndPopulateAlt(client, admin);

  process.exit();
}

main();

async function createAndPopulateAlt(client: MangoClient, admin: Keypair) {
  let group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

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
      let sig = await client.sendAndConfirmTransaction([createIx[0]]);
      console.log(
        `...created ALT ${createIx[1]} https://explorer.solana.com/tx/${sig}`,
      );

      console.log(`ALT: set at index 0 for group...`);
      sig = await client.altSet(group, createIx[1], 0);
      console.log(`...https://explorer.solana.com/tx/${sig}`);

      group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
    } catch (error) {
      console.log(error);
    }
  }

  // Extend using mango v4 relevant pub keys
  try {
    let bankAddresses = Array.from(group.banksMapByMint.values())
      .flat()
      .map((bank) => [bank.publicKey, bank.oracle, bank.vault])
      .flat()
      .concat(
        Array.from(group.banksMapByMint.values())
          .flat()
          .map((mintInfo) => mintInfo.publicKey),
      );

    let serum3MarketAddresses = Array.from(
      group.serum3MarketsMapByExternal.values(),
    )
      .flat()
      .map((serum3Market) => serum3Market.publicKey);

    let serum3ExternalMarketAddresses = Array.from(
      group.serum3ExternalMarketsMap.values(),
    )
      .flat()
      .map((serum3ExternalMarket) => [
        serum3ExternalMarket.publicKey,
        serum3ExternalMarket.bidsAddress,
        serum3ExternalMarket.asksAddress,
      ])
      .flat();

    let perpMarketAddresses = Array.from(
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

    async function extendTable(addresses: PublicKey[]) {
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
      const sig = await client.sendAndConfirmTransaction([extendIx]);
      console.log(`https://explorer.solana.com/tx/${sig}`);
    }

    console.log(`ALT: extending using mango v4 relevant public keys`);
    await extendTable(bankAddresses);
    await extendTable(serum3MarketAddresses);
    await extendTable(serum3ExternalMarketAddresses);
    await extendTable(perpMarketAddresses);
  } catch (error) {
    console.log(error);
  }
}
