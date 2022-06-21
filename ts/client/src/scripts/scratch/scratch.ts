import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Market } from '@project-serum/serum';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import * as os from 'os';

import { MangoClient } from '../../client';
import { MANGO_V4_ID } from '../../constants';

const main = async () => {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(os.homedir() + '/.config/solana/admin.json', 'utf-8'),
      ),
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

  const btcMint = new PublicKey('3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU');
  const usdcMint = new PublicKey(
    'EmXq3Ni9gfudTiyNKzzYvpnQqnJEMRw2ttnVXoJXjLo1',
  );
  const serumProgramId = new PublicKey(
    'DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY',
  );

  const market = await Market.findAccountsByMints(
    connection,
    btcMint,
    usdcMint,
    serumProgramId,
  );

  console.log('market', market);
};

main();
