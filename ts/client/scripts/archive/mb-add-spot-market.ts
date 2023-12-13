import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
dotenv.config();

//
// (untested?) script which closes a mango account cleanly, first closes all positions, withdraws all tokens and then closes it
//
async function addSpotMarket() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // fetch group
  const groupPk = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';
  const group = await client.getGroup(new PublicKey(groupPk));
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const eth_openbook_mkt = 'FZxi3yWkE5mMjyaZj6utmYL54QQYfMCKMcLaQZq4UwnA';
  const eth_mint = '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs';
  const usdc_mint = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

  const signature = await client.serum3RegisterMarket(
    group,
    new PublicKey(eth_openbook_mkt),
    group.getFirstBankByMint(new PublicKey(eth_mint)),
    group.getFirstBankByMint(new PublicKey(usdc_mint)),
    1, // market index
    'ETH/USDC',
    0.5,
  );

  console.log('Tx Successful:', signature);

  process.exit();
}

async function main() {
  await addSpotMarket();
}

main();
