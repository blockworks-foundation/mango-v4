import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

export const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
]);

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

  const group = await client.getGroupForAdmin(admin.publicKey);
  console.log(`Group ${group.publicKey}`);

  let sig;

  // close stub oracle
  const usdcDevnetMint = new PublicKey(DEVNET_MINTS.get('USDC')!);
  try {
    const usdcDevnetOracle = await client.getStubOracle(
      group,
      usdcDevnetMint,
    )[0];
    let sig = await client.closeStubOracle(group, usdcDevnetOracle.publicKey);
    console.log(
      `Closed USDC stub oracle, sig https://explorer.solana.com/address/${sig}?cluster=devnet`,
    );
  } catch (error) {
    console.error(error);
  }

  // close all bank
  for (const bank of group.banksMap.values()) {
    try {
      sig = await client.tokenDeregister(group, bank.name);
      console.log(
        `Removed token ${bank.name}, sig https://explorer.solana.com/address/${sig}?cluster=devnet`,
      );
    } catch (error) {
      console.error(error);
    }
  }

  // deregister all serum markets
  for (const market of group.serum3MarketsMap.values()) {
    try {
      sig = await client.serum3deregisterMarket(group, market.name);
      console.log(
        `Deregistered serum market ${market.name}, sig https://explorer.solana.com/address/${sig}?cluster=devnet`,
      );
    } catch (error) {
      console.error(error);
    }
  }

  // close all perp markets
  for (const market of group.perpMarketsMap.values()) {
    try {
      sig = await client.perpCloseMarket(group, market.name);
      console.log(
        `Closed perp market ${market.name}, sig https://explorer.solana.com/address/${sig}?cluster=devnet`,
      );
    } catch (error) {
      console.error(error);
    }
  }

  // finally, close the group
  try {
    sig = await client.closeGroup(group);
    console.log(
      `Closed group, sig https://explorer.solana.com/address/${sig}?cluster=devnet`,
    );
  } catch (error) {
    console.error(error);
  }

  process.exit();
}

main();
