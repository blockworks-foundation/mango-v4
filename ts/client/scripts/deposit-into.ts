import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { BN } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, MANGO_ACCOUNT, MINT, NATIVE_AMOUNT } =
  process.env;

const CLIENT_USER = MB_PAYER_KEYPAIR;
const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

async function buildClient(): Promise<MangoClient> {
  const clientKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(CLIENT_USER!, 'utf-8'))),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const clientWallet = new Wallet(clientKeypair);
  const clientProvider = new AnchorProvider(connection, clientWallet, options);

  return await MangoClient.connect(
    clientProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );
}

async function main(): Promise<void> {
  const client = await buildClient();
  const mangoAccount = await client.getMangoAccount(
    new PublicKey(MANGO_ACCOUNT!),
  );
  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const mintPk = new PublicKey(MINT!);

  const rs = await client.tokenDepositNative(
    group,
    mangoAccount,
    mintPk,
    new BN(NATIVE_AMOUNT!),
    false,
    true,
  );
  console.log(rs.signature);
}

try {
  main();
} catch (error) {
  console.log(error);
}
