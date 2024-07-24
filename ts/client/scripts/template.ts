import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../src/client';

async function main(): Promise<void> {
  const client = await MangoClient.connectDefault(process.env.MB_CLUSTER_URL!);
  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );
}

main();
