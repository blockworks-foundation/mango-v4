import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { TokenAccount } from '../../accounts/mangoAccount';
import { MangoClient } from '../../client';
import { MANGO_V4_ID } from '../../constants';

//
// An example for users based on high level api i.e. the client
// Create
// process.env.USER_KEYPAIR - mango account owner keypair path
// process.env.ADMIN_KEYPAIR - group admin keypair path (useful for automatically finding the group)
//
async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForAdmin(admin.publicKey);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
    0,
    'my_mango_account',
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);

  // log users tokens
  for (const token of mangoAccount.tokens) {
    if (token.tokenIndex == TokenAccount.TokenIndexUnset) continue;
    console.log(token.toString());
  }

  process.exit();
}

main();
