import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoAccount } from '../../src/accounts/mangoAccount';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

const TESTNET_MINTS = new Map([
  ['USDC', 'AkdEhBMvaDD1UbGMD3Hxnr3h5PEL2R8PaCDAssCN28WV'],
]);

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://testnet.dev2.eclipsenetwork.xyz',
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
    'testnet',
    MANGO_V4_ID['testnet'],
    {
      idsSource: 'get-program-accounts',
    },
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`Group ${group.publicKey}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  let mangoAccount = (await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  )) as MangoAccount;
  if (!mangoAccount) {
    await client.createMangoAccount(group, 0);
    mangoAccount = (await client.getMangoAccountForOwner(
      group,
      user.publicKey,
      0,
    )) as MangoAccount;
  }
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);

  // deposit USDC
  const sig = await client.tokenDeposit(
    group,
    mangoAccount,
    new PublicKey(TESTNET_MINTS.get('USDC')!),
    50,
  );
  console.log(
    `deposited token, https://explorer.dev.eclipsenetwork.xyz/tx/${sig.signature}?cluster=testnet`,
  );

  process.exit();
}

main();
