import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { getLiquidationBatches, getRiskStats } from '../src/risk';

const { MB_CLUSTER_URL } = process.env;

const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

async function buildClient(): Promise<MangoClient> {
  const clientKeypair = new Keypair();

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
  const group = await client.getGroup(new PublicKey(GROUP_PK));
  try {
    console.log(JSON.stringify(await getRiskStats(client, group), null, 2));
    console.log(
      JSON.stringify(await getLiquidationBatches(client, group), null, 2),
    );
  } catch (error) {
    console.log(error);
  }
}

main();
