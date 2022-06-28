import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
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

  const group = await client.getGroupForAdmin(admin.publicKey);
  console.log(`Group ${group.publicKey}`);

  let sig;

  // close stub oracle
  const usdcDevnetMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  const usdcDevnetOracle = await client.getStubOracle(group, usdcDevnetMint)[0];
  sig = await client.closeStubOracle(group, usdcDevnetOracle.publicKey);
  console.log(
    `Closed USDC stub oracle, sig https://explorer.solana.com/address/${sig}`,
  );

  // close all bank
  for (const bank of group.banksMap.values()) {
    sig = await client.tokenDeregister(group, bank.name);
    console.log(
      `Removed token ${bank.name}, sig https://explorer.solana.com/address/${sig}`,
    );
  }

  // deregister all serum markets
  for (const market of group.serum3MarketsMap.values()) {
    sig = await client.serum3deregisterMarket(group, market.name);
    console.log(
      `Deregistered serum market ${market.name}, sig https://explorer.solana.com/address/${sig}`,
    );
  }

  // close all perp markets
  for (const market of group.perpMarketsMap.values()) {
    sig = await client.perpCloseMarket(group, market.name);
    console.log(
      `Closed perp market ${market.name}, sig https://explorer.solana.com/address/${sig}`,
    );
  }

  // finally, close the group
  sig = await client.closeGroup(group);
  console.log(`Closed group, sig https://explorer.solana.com/address/${sig}`);

  process.exit();
}

main();
