import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { PerpMarketIndex } from '../../../src/accounts/perp';
import { MangoClient } from '../../../src/client';
import { DefaultTokenRegisterParams } from '../../../src/clientIxParamBuilder';
import { MANGO_V4_ID } from '../../../src/constants';

const DEVNET_SERUM3_MARKETS = new Map([
  ['SOL/USDC', '6xYbSQyhajUqyatJDdkonpj7v41bKeEBWpf7kwRh5X7A'],
]);
const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'],
  ['USDT', 'DAwBSXe6w9g37wdE2tCrFbho3QHKZi4PjuBytQCULap2'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
]);
const DEVNET_ORACLES = new Map([
  ['SOL', 'J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix'],
]);

const GROUP_NUM = 2814;

async function main(): Promise<void> {
  let sig;

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
    {
      idsSource: 'get-program-accounts',
    },
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

  // stub usdc oracle + register token 0
  console.log(`Registering USDC...`);
  const usdcDevnetMint = new PublicKey(DEVNET_MINTS.get('USDC')!);
  try {
    sig = await client.stubOracleCreate(group, insuranceMint, 1.0);
    const usdcDevnetOracle = (
      await client.getStubOracle(group, insuranceMint)
    )[0];
    console.log(
      `...registered stub oracle ${usdcDevnetOracle}, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
    );

    sig = await client.tokenRegister(
      group,
      usdcDevnetMint,
      usdcDevnetOracle.publicKey,
      0, // tokenIndex
      'USDC',
      {
        ...DefaultTokenRegisterParams,
        oracleConfig: {
          confFilter: 10000,
          maxStalenessSlots: null,
        },
      },
    );
    await group.reloadAll(client);
    const bank = group.getFirstBankByMint(usdcDevnetMint);
    console.log(
      `...registered token bank ${bank.publicKey}, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
    );
    await group.reloadAll(client);
    // eslint-disable-next-line
  } catch (error) {}

  // register token 4
  console.log(`Registering SOL...`);
  const solDevnetMint = new PublicKey(DEVNET_MINTS.get('SOL')!);
  const solDevnetOracle = new PublicKey(DEVNET_ORACLES.get('SOL')!);
  try {
    sig = await client.tokenRegister(
      group,
      solDevnetMint,
      solDevnetOracle,
      4, // tokenIndex
      'SOL',
      {
        ...DefaultTokenRegisterParams,
        oracleConfig: {
          confFilter: 10000,
          maxStalenessSlots: null,
        },
      },
    );
    await group.reloadAll(client);
    const bank = group.getFirstBankByMint(solDevnetMint);
    console.log(
      `...registered token bank ${bank.publicKey}, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
    );
  } catch (error) {
    console.log(error);
  }

  console.log(`Registering USDT...`);
  const usdtDevnetMint = new PublicKey(DEVNET_MINTS.get('USDT')!);
  const usdcDevnetOracle = (
    await client.getStubOracle(group, insuranceMint)
  )[0];
  try {
    sig = await client.tokenRegister(
      group,
      usdtDevnetMint,
      usdcDevnetOracle.publicKey,
      5, // tokenIndex
      'USDT',
      {
        ...DefaultTokenRegisterParams,
        oracleConfig: {
          confFilter: 10000,
          maxStalenessSlots: null,
        },
      },
    );
    await group.reloadAll(client);
    const bank = group.getFirstBankByMint(solDevnetMint);
    console.log(
      `...registered token bank ${bank.publicKey}, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
    );
  } catch (error) {
    console.log(error);
  }

  // register serum market
  console.log(`Registering serum3 markets...`);

  const serumMarketExternalPk = new PublicKey(
    DEVNET_SERUM3_MARKETS.get('SOL/USDC')!,
  );
  try {
    sig = await client.serum3RegisterMarket(
      group,
      serumMarketExternalPk,
      group.getFirstBankByMint(solDevnetMint),
      group.getFirstBankByMint(insuranceMint),
      0,
      'SOL/USDC',
    );
    await group.reloadAll(client);
    const serum3Market = group.getSerum3MarketByExternalMarket(
      serumMarketExternalPk,
    );
    console.log(
      `...registered serum market ${serum3Market.publicKey}, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
    );
  } catch (error) {
    console.log(error);
  }

  // register perp market
  let count = 0;
  console.log(`Registering perp market...`);
  for (const market of ['SOL-PERP1', 'SOL-PERP2', 'SOL-PERP3']) {
    count = count + 1;
    try {
      sig = await client.perpCreateMarket(
        group,
        new PublicKey(DEVNET_ORACLES.get('SOL')!),
        count,
        market,
        { confFilter: 10000, maxStalenessSlots: null },
        6,
        10,
        100,
        0.975,
        0.95,
        1.025,
        1.05,
        0.95,
        0.9,
        0.012,
        0.0002,
        0.0,
        0,
        0.05,
        0.05,
        100,
        true,
        1000,
        1000000,
        0.05,
        0,
        1.0,
        2 * 60 * 60,
        0.025,
      );
      await group.reloadAll(client);
      const perpMarket = group.getPerpMarketByMarketIndex(
        count as PerpMarketIndex,
      );
      console.log(
        `...registered perp market ${perpMarket.publicKey}, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
      );
    } catch (error) {
      console.log(error);
    }
  }

  process.exit();
}

main();
