import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

dotenv.config();

async function addSpotMarket() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // fetch group
  const groupPk = '7SDejCUPsF3g59GgMsmvxw8dJkkJbT3exoH4RZirwnkM';
  const group = await client.getGroup(new PublicKey(groupPk));
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const baseMint = new PublicKey('So11111111111111111111111111111111111111112');
  const quoteMint = new PublicKey(
    '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN',
  ); //devnet usdc

  const marketPubkey = new PublicKey(
    '85o8dcTxhuV5N3LFkF1pKoCBsXhdekgdQeJ8zGEgnBwP',
  );

  const signature = await client.openbookV2RegisterMarket(
    group,
    marketPubkey,
    group.getFirstBankByMint(baseMint),
    group.getFirstBankByMint(quoteMint),
    1,
    'SOL/USDC',
    0,
  );
  console.log('Tx Successful:', signature);

  process.exit();
}

async function main() {
  await addSpotMarket();
}

main();
