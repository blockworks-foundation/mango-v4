import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// (unfinished?) script which shows how to use the margin 1 ix
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
  const group = await client.getGroupForCreator(admin.publicKey, 0);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  if (false) {
    // deposit and withdraw
    console.log(`Depositing...50 USDC`);
    await client.tokenDeposit(group, mangoAccount, 'USDC', 50);
    await mangoAccount.reload(client, group);

    console.log(`Depositing...0.0005 BTC`);
    await client.tokenDeposit(group, mangoAccount, 'BTC', 0.0005);
    await mangoAccount.reload(client, group);
  }
  try {
    const sig = await client.marginTrade({
      group: group,
      mangoAccount: mangoAccount,
      inputToken: 'USDC',
      amountIn: 0.001,
      outputToken: 'SOL',
      slippage: 1,
    });
    console.log(
      `sig https://explorer.solana.com/address/${sig}?cluster=devnet`,
    );
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

main();
