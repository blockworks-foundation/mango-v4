import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  AddressLookupTableProgram,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';
import fs from 'fs';
import { PerpMarketIndex } from '../../src/accounts/perp';
import { MangoClient } from '../../src/client';
import { DefaultTokenRegisterParams } from '../../src/clientIxParamBuilder';
import { MANGO_V4_ID } from '../../src/constants';
import { buildVersionedTx } from '../../src/utils';

//
// An example for admins based on high level api i.e. the client
// Depoys a new mango group to devnet, registers 4 tokens, and 1 serum3 spot market
//
// process.env.ADMIN_KEYPAIR - group admin keypair path
// to create a new admin keypair:
// * solana-keygen new --outfile ~/.config/solana/admin.json
// * solana airdrop 1  -k ~/.config/solana/admin.json
//

// https://github.com/blockworks-foundation/mango-client-v3/blob/main/src/serum.json#L70
const DEVNET_SERUM3_MARKETS = new Map([
  ['SOL/USDC', '6xYbSQyhajUqyatJDdkonpj7v41bKeEBWpf7kwRh5X7A'],
]);
const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
  ['SOL', 'So11111111111111111111111111111111111111112'],
]);
const DEVNET_ORACLES = new Map([
  ['SOL', 'J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix'],
  ['BTC', 'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J'],
  ['ETH', 'EdVCmQ9FSPcVe5YySXDPCRmc8aDQLKJ9xvYBMZPie1Vw'],
]);

// TODO: should these constants be baked right into client.ts or even program?
const NET_BORROWS_LIMIT_NATIVE = 1 * Math.pow(10, 7) * Math.pow(10, 6);

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

async function main() {
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
    sig = await client.stubOracleCreate(group, usdcDevnetMint, 1.0);
    const usdcDevnetOracle = (
      await client.getStubOracle(group, usdcDevnetMint)
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
        initAssetWeight: 1,
        maintAssetWeight: 1,
        initLiabWeight: 1,
        maintLiabWeight: 1,
        liquidationFee: 0,
        netBorrowLimitPerWindowQuote: NET_BORROWS_LIMIT_NATIVE,
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

  // register token 1
  console.log(`Registering SOL...`);
  const solDevnetMint = new PublicKey(DEVNET_MINTS.get('SOL')!);
  const solDevnetOracle = new PublicKey(DEVNET_ORACLES.get('SOL')!);
  try {
    sig = await client.tokenRegister(
      group,
      solDevnetMint,
      solDevnetOracle,
      1, // tokenIndex
      'SOL',
      {
        ...DefaultTokenRegisterParams,
        maintAssetWeight: 0.9,
        initAssetWeight: 0.8,
        maintLiabWeight: 1.1,
        initLiabWeight: 1.2,
        liquidationFee: 0.05,
        netBorrowLimitPerWindowQuote: NET_BORROWS_LIMIT_NATIVE,
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

  console.log(
    `Editing group, setting existing admin as fastListingAdmin to be able to add MNGO truslessly...`,
  );
  sig = await client.groupEdit(
    group,
    group.admin,
    group.admin,
    undefined,
    undefined,
  );
  console.log(
    `...edited group, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
  );

  // register serum market
  console.log(`Registering serum3 market...`);
  const serumMarketExternalPk = new PublicKey(
    DEVNET_SERUM3_MARKETS.get('SOL/USDC')!,
  );
  try {
    sig = await client.serum3RegisterMarket(
      group,
      serumMarketExternalPk,
      group.getFirstBankByMint(solDevnetMint),
      group.getFirstBankByMint(usdcDevnetMint),
      0,
      'SOL/USDC',
      0.5,
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
  console.log(`Registering perp market...`);
  try {
    sig = await client.perpCreateMarket(
      group,
      new PublicKey(DEVNET_ORACLES.get('BTC')!),
      0,
      'BTC-PERP',
      DefaultTokenRegisterParams.oracleConfig,
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
    const perpMarket = group.getPerpMarketByMarketIndex(0 as PerpMarketIndex);
    console.log(
      `...registered perp market ${perpMarket.publicKey}, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
    );
  } catch (error) {
    console.log(error);
  }
  const perpMarkets = await client.perpGetMarkets(group);
  console.log(`...created perp market ${perpMarkets[0].publicKey}`);

  if (group.addressLookupTables[0].equals(PublicKey.default)) {
    try {
      console.log(`ALT...`);
      const createIx = AddressLookupTableProgram.createLookupTable({
        authority: admin.publicKey,
        payer: admin.publicKey,
        recentSlot: await connection.getSlot('finalized'),
      });
      const createTx = await buildVersionedTx(
        client.program.provider as AnchorProvider,
        [createIx[0]],
      );
      sig = await connection.sendTransaction(createTx);
      console.log(
        `...created ALT ${createIx[1]} https://explorer.solana.com/tx/${sig}?cluster=devnet`,
      );

      sig = await client.altSet(
        group,
        new PublicKey('EmN5RjHUFsoag7tZ2AyBL2N8JrhV7nLMKgNbpCfzC81D'),
        0,
      );
      console.log(
        `...set at index 0 for group https://explorer.solana.com/tx/${sig}?cluster=devnet`,
      );

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
      console.log(
        `...extended ALT with pks, https://explorer.solana.com/tx/${sig}?cluster=devnet`,
      );
    } catch (error) {
      console.log(error);
    }
  }

  // await client.serum3EditMarket(group, 0 as MarketIndex, false, false);

  // const perpMarket = group.getPerpMarketByMarketIndex(0 as PerpMarketIndex);
  // const params = Builder(NullPerpEditParams)
  //   .reduceOnly(true)
  //   .forceClose(true)
  //   .build();
  // await client.perpEditMarket(group, 0 as PerpMarketIndex, params);

  // const params = Builder(NullTokenEditParams)
  //   .reduceOnly(2)
  //   .forceClose(true)
  //   .build();
  // await client.tokenEdit(
  //   group,
  //   group.banksMapByName.get('SOL')![0].mint,
  //   params,
  // );

  process.exit();
}

main();
