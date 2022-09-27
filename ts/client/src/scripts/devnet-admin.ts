import { AnchorProvider, Wallet } from '@project-serum/anchor';
import {
  AddressLookupTableProgram,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { buildVersionedTx } from '../utils';

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
  ['ETH/USDC', 'BkAraCyL9TTLbeMY3L1VWrPcv32DvSi5QDDQjik1J6Ac'],
  ['SRM/USDC', '249LDNPLLL29nRq8kjBTg9hKdXMcZf4vK2UvxszZYcuZ'],
]);
const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['ORCA', 'orcarKHSqC5CDDsGbho8GKvwExejWHxTqGzXgcewB9L'],
  ['MNGO', 'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC'],
  ['ETH', 'Cu84KB3tDL6SbFgToHMLYVDJJXdJjenNzSKikeAvzmkA'],
  ['SRM', 'AvtB6w9xboLwA145E221vhof5TddhqsChYcx7Fy3xVMH'],
]);
const DEVNET_ORACLES = new Map([
  ['BTC', 'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J'],
  ['SOL', 'J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix'],
  ['ORCA', 'A1WttWF7X3Rg6ZRpB2YQUFHCRh1kiXV8sKKLV3S9neJV'],
  ['MNGO', '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o'],
  ['ETH', 'EdVCmQ9FSPcVe5YySXDPCRmc8aDQLKJ9xvYBMZPie1Vw'],
  ['SRM', '992moaMQKs32GKZ9dxi8keyM2bUmbrwBZpK4p2K6X5Vs'],
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
    {},
    'get-program-accounts',
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

  // register token 7
  console.log(`Registering ETH...`);
  const ethDevnetMint = new PublicKey(DEVNET_MINTS.get('ETH')!);
  const ethDevnetOracle = new PublicKey(DEVNET_ORACLES.get('ETH')!);
  try {
    await client.tokenRegister(
      group,
      ethDevnetMint,
      ethDevnetOracle,
      0.1,
      7, // tokenIndex
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
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  // register token 5
  console.log(`Registering SRM...`);
  const srmDevnetMint = new PublicKey(DEVNET_MINTS.get('SRM')!);
  const srmDevnetOracle = new PublicKey(DEVNET_ORACLES.get('SRM')!);
  try {
    await client.tokenRegister(
      group,
      srmDevnetMint,
      srmDevnetOracle,
      0.1,
      5, // tokenIndex
      'SRM',
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
    await group.reloadAll(client);
  } catch (error) {
    console.log(error);
  }

  console.log(
    `Editing group, setting existing admin as fastListingAdmin to be able to add MNGO truslessly...`,
  );
  let sig = await client.groupEdit(
    group,
    group.admin,
    group.admin,
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
  group.consoleLogBanks();

  // register serum market
  console.log(`Registering serum3 market...`);
  let serumMarketExternalPk = new PublicKey(
    DEVNET_SERUM3_MARKETS.get('BTC/USDC')!,
  );
  try {
    await client.serum3RegisterMarket(
      group,
      serumMarketExternalPk,
      group.getFirstBankByMint(btcDevnetMint),
      group.getFirstBankByMint(usdcDevnetMint),
      0,
      'BTC/USDC',
    );
  } catch (error) {
    console.log(error);
  }
  const markets = await client.serum3GetMarkets(
    group,
    group.getFirstBankByMint(btcDevnetMint).tokenIndex,
    group.getFirstBankByMint(usdcDevnetMint).tokenIndex,
  );
  console.log(`...registered serum3 market ${markets[0].publicKey}`);

  serumMarketExternalPk = new PublicKey(DEVNET_SERUM3_MARKETS.get('ETH/USDC')!);
  try {
    await client.serum3RegisterMarket(
      group,
      serumMarketExternalPk,
      group.getFirstBankByMint(ethDevnetMint),
      group.getFirstBankByMint(usdcDevnetMint),
      1,
      'ETH/USDC',
    );
  } catch (error) {
    console.log(error);
  }

  serumMarketExternalPk = new PublicKey(DEVNET_SERUM3_MARKETS.get('SRM/USDC')!);
  try {
    await client.serum3RegisterMarket(
      group,
      serumMarketExternalPk,
      group.getFirstBankByMint(srmDevnetMint),
      group.getFirstBankByMint(usdcDevnetMint),
      2,
      'SRM/USDC',
    );
  } catch (error) {
    console.log(error);
  }

  // register perp market
  console.log(`Registering perp market...`);
  try {
    await client.perpCreateMarket(
      group,
      btcDevnetOracle,
      0,
      'BTC-PERP',
      0.1,
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
      0,
      0.05,
      0.05,
      100,
      true,
      true,
      0,
      0,
      0,
    );
    console.log('done');
  } catch (error) {
    console.log(error);
  }
  const perpMarkets = await client.perpGetMarkets(group);
  console.log(`...created perp market ${perpMarkets[0].publicKey}`);

  //
  // edit
  //

  if (true) {
    console.log(`Editing USDC...`);
    try {
      let sig = await client.tokenEdit(
        group,
        usdcDevnetMint,
        usdcDevnetOracle.publicKey,
        0.1,
        null,
        {
          adjustmentFactor: 0.004,
          util0: 0.7,
          rate0: 0.1,
          util1: 0.85,
          rate1: 0.2,
          maxRate: 2.0,
        },
        0.005,
        0.0005,
        1,
        1,
        1,
        1,
        0,
      );
      console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
      await group.reloadAll(client);
      console.log(group.getFirstBankByMint(btcDevnetMint).toString());
    } catch (error) {
      throw error;
    }

    console.log(`Editing BTC...`);
    try {
      let sig = await client.tokenEdit(
        group,
        usdcDevnetMint,
        usdcDevnetOracle.publicKey,
        0.1,
        null,
        {
          adjustmentFactor: 0.004,
          util0: 0.7,
          rate0: 0.1,
          util1: 0.85,
          rate1: 0.2,
          maxRate: 2.0,
        },
        0.005,
        0.0005,
        0.9,
        0.8,
        1.1,
        1.2,
        0.05,
      );
      console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
      await group.reloadAll(client);
      console.log(group.getFirstBankByMint(btcDevnetMint).toString());
    } catch (error) {
      throw error;
    }

    console.log(`Editing SOL...`);
    try {
      let sig = await client.tokenEdit(
        group,
        usdcDevnetMint,
        usdcDevnetOracle.publicKey,
        0.1,
        null,
        {
          adjustmentFactor: 0.004,
          util0: 0.7,
          rate0: 0.1,
          util1: 0.85,
          rate1: 0.2,
          maxRate: 2.0,
        },
        0.005,
        0.0005,
        0.9,
        0.8,
        1.1,
        1.2,
        0.05,
      );
      console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
      await group.reloadAll(client);
      console.log(group.getFirstBankByMint(btcDevnetMint).toString());
    } catch (error) {
      throw error;
    }

    console.log(`Editing BTC-PERP...`);
    try {
      let sig = await client.perpEditMarket(
        group,
        'BTC-PERP',
        btcDevnetOracle,
        0.1,
        6,
        0.975,
        0.95,
        1.025,
        1.05,
        0.012,
        0.0002,
        0.0,
        0,
        0.05,
        0.05,
        100,
        true,
        true,
        0,
        0,
        0,
      );
      console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
      await group.reloadAll(client);
      console.log(group.getFirstBankByMint(btcDevnetMint).toString());
    } catch (error) {
      throw error;
    }
  }

  if (
    // true
    group.addressLookupTables[0].equals(PublicKey.default)
  ) {
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
        `...created ALT ${createIx[1]} https://explorer.solana.com/tx/${sig}?cluster=devnet`,
      );

      console.log(`ALT: set at index 0 for group...`);
      sig = await client.altSet(
        group,
        new PublicKey('EmN5RjHUFsoag7tZ2AyBL2N8JrhV7nLMKgNbpCfzC81D'),
        0,
      );
      console.log(`...https://explorer.solana.com/tx/${sig}?cluster=devnet`);

      // Extend using a mango v4 program ix
      // Throws > Instruction references an unknown account 11111111111111111111111111111111 atm
      //
      console.log(
        `ALT: extending using mango v4 program with bank publick keys and oracles`,
      );
      // let sig = await client.altExtend(
      //   group,
      //   new PublicKey('EmN5RjHUFsoag7tZ2AyBL2N8JrhV7nLMKgNbpCfzC81D'),
      //   0,
      //   Array.from(group.banksMapByMint.values())
      //     .flat()
      //     .map((bank) => [bank.publicKey, bank.oracle])
      //     .flat(),
      // );
      // console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);

      // TODO decide on what keys should go in
      console.log(`ALT: extending manually with bank publick keys and oracles`);
      const extendIx = AddressLookupTableProgram.extendLookupTable({
        lookupTable: createIx[1],
        payer: admin.publicKey,
        authority: admin.publicKey,
        addresses: Array.from(group.banksMapByMint.values())
          .flat()
          .map((bank) => [bank.publicKey, bank.oracle])
          .flat(),
      });
      const extendTx = await buildVersionedTx(
        client.program.provider as AnchorProvider,
        [extendIx],
      );
      sig = await client.program.provider.connection.sendTransaction(extendTx);
      console.log(`https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    } catch (error) {
      console.log(error);
    }
  }

  try {
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

main();
