import { Provider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from './client';
import {
  DEVNET_MINTS,
  DEVNET_ORACLES,
  DEVNET_SERUM3_MARKETS,
  DEVNET_SERUM3_PROGRAM_ID,
} from './constants';

//
// An example for admins based on high level api i.e. the client
//
async function main() {
  const options = Provider.defaultOptions();
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
  const adminProvider = new Provider(connection, adminWallet, options);
  const client = await MangoClient.connect(adminProvider, true);

  // group
  try {
    await client.createGroup();
  } catch (error) {}
  const group = await client.getGroup(admin.publicKey);
  console.log(`Group ${group.publicKey}`);

  // register token 0
  const btcDevnetMint = new PublicKey(DEVNET_MINTS['BTC']);
  const btcDevnetOracle = new PublicKey(DEVNET_ORACLES['BTC']);
  try {
    await client.registerToken(group, btcDevnetMint, btcDevnetOracle, 0);
  } catch (error) {}

  // stub oracle + register token 1
  const usdcDevnetMint = new PublicKey(DEVNET_MINTS['USDC']);
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
    );
  } catch (error) {}

  // log tokens/banks
  const banks = await client.getBanksForGroup(group);
  for (const bank of banks) {
    console.log(
      `Bank ${bank.tokenIndex} ${bank.publicKey}, mint ${bank.mint}, oracle ${bank.oracle}`,
    );
  }

  // register serum market
  const serumMarketExternalPk = new PublicKey(
    DEVNET_SERUM3_MARKETS['BTC/USDC'],
  );
  try {
  } catch (error) {
    await client.serum3RegisterMarket(
      group,
      DEVNET_SERUM3_PROGRAM_ID,
      serumMarketExternalPk,
      banks[0],
      banks[1],
      0,
    );
  }
  const markets = await client.serum3GetMarket(
    group,
    banks[0].tokenIndex,
    banks[1].tokenIndex,
  );
  console.log(`Serum3 market ${markets[0].publicKey}`);

  process.exit();
}

main();
