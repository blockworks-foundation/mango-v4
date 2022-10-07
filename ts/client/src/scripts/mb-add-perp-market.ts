import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Serum3Side } from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

const MAINNET_ORACLES = new Map([
  ['USDT', '3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL'],
  ['BTC', 'GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'],
  ['ETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['soETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['SOL', 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'],
  ['MSOL', 'E4v1BBgoso9s64TQvmyownAVJbhbEPGyzA3qn4n46qj9'],
  ['MNGO', '79wm3jjcPr6RaNQ4DGvP5KxG1mNd3gEBsg6FsNVFezK4'],
]);

const PAYER_KEYPAIR = process.env.MB_PAYER_KEYPAIR || '';

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
  // const userWallet = new Wallet(Keypair.generate());
  // const userProvider = new AnchorProvider(connection, userWallet, options);
  // console.log(`User ${userWallet.publicKey.toBase58()}`);

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(PAYER_KEYPAIR, 'utf-8'))),
  );
  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const btcMainnetOracle = new PublicKey(MAINNET_ORACLES.get('BTC')!);
  console.log(`Registering perp market...`);
  try {
    await client.perpCreateMarket(
      group,
      btcMainnetOracle,
      0,
      'BTC-PERP',
      0.1,
      6,
      0,
      1,
      10,
      0.975,
      0.95,
      1.025,
      1.05,
      0.012,
      0.0002,
      0.0,
      0.05,
      0.05,
      100,
      false,
      true,
    );
    console.log('done');
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

async function main() {
  await viewUnownedAccount(process.env.MB_USER2_KEYPAIR || '');
}

main();
