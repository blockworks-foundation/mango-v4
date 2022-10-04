import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Serum3Side } from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// (untested?) script which closes a mango account cleanly, first closes all positions, withdraws all tokens and then closes it
//
async function viewUnownedAccount(userKeypairFile: string) {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.rpcpool.com/0f9acc0d45173b51bf7d7e09c1e5',
    options,
  );

  // user
  const userWallet = new Wallet(Keypair.generate());
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR || '', 'utf-8')),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const x = await client.getMangoAccount(
    new PublicKey('6cTqJrSzQZWGEeHHePqFuJV4Kf54YDVfSamdCrT3agw6'),
  );
  const y = await x.reloadAccountData(client, group);

  process.exit();
}

async function main() {
  await viewUnownedAccount(process.env.MB_USER2_KEYPAIR || '');
}

main();
