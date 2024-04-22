import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { OpenBookV2Client } from '@openbook-dex/openbook-v2';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { sendTransaction } from '../src/utils/rpc';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;

async function run() {
  const conn = new Connection(CLUSTER_URL!, 'processed');
  const kp = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const wallet = new Wallet(kp);

  const provider = new AnchorProvider(conn, wallet, {});
  const client: OpenBookV2Client = new OpenBookV2Client(provider, undefined, {
    prioritizationFee: 10_000,
  });

  const ix = await client.createMarketIx(
    wallet.publicKey,
    'sol-apr22/usdc',
    new PublicKey('So11111111111111111111111111111111111111112'), // sol
    new PublicKey('8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'), // usdc
    new BN(100),
    new BN(100),
    new BN(100),
    new BN(100),
    new BN(100),
    null,
    null,
    null,
    null,
    provider.wallet.publicKey,
  );

  const res = await sendTransaction(
    client.program.provider as AnchorProvider,
    ix[0],
    [],
    {
      prioritizationFee: 1,
      additionalSigners: ix[1] as any,
    },
  );

  console.log(res);
}

run();
