import { AnchorProvider, Wallet } from '@project-serum/anchor';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  Token,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import { Connection, Keypair, PublicKey, Transaction } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { getAssociatedTokenAddress } from '../utils';

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
  // const usdcMainnetMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  // const usdcMainnetOracle = await client.getStubOracle(
  //   group,
  //   usdcMainnetMint,
  // )[0];
  // console.log(usdcMainnetOracle);
  // sig = await client.closeStubOracle(group, usdcMainnetOracle.publicKey);
  // sig = await client.closeStubOracle(
  //   group,
  //   new PublicKey('A9XhGqUUjV992cD36qWDY8wDiZnGuCaUWtSE3NGXjDCb'),
  // );
  // console.log(
  //   `Closed USDC stub oracle, sig https://explorer.solana.com/address/${sig}`,
  // );

  let tx = new Transaction();
  tx.add(
    Token.createAssociatedTokenAccountInstruction(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      new PublicKey('So11111111111111111111111111111111111111112'),
      await getAssociatedTokenAddress(
        new PublicKey('So11111111111111111111111111111111111111112'),
        admin.publicKey,
      ),
      admin.publicKey,
      admin.publicKey,
    ),
  );
  await client.program.provider.sendAndConfirm(tx);

  // close all bank
  for (const bank of group.banksMap.values()) {
    console.log(`Removing token ${bank.name}...`);
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
