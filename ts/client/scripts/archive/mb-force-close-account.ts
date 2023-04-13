import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../../src/accounts/group';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK;
const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, MB_USER_KEYPAIR, MB_USER4_KEYPAIR } =
  process.env;

async function buildUserClient(
  userKeypair: string,
): Promise<[MangoClient, Group, Keypair]> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(userKeypair, 'utf-8'))),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);

  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  return [client, group, user];
}

async function forceCloseUserAccount() {
  const result = await buildUserClient(MB_PAYER_KEYPAIR!);
  const client = result[0];
  const group = result[1];
  const mangoAccount = await client.getMangoAccount(
    new PublicKey(MANGO_ACCOUNT_PK!),
  );
  await client.closeMangoAccount(group, mangoAccount, true);
  process.exit();
}

async function main() {
  await forceCloseUserAccount();
}

main();
