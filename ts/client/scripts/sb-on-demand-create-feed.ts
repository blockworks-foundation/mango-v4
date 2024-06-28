import {
  Cluster,
  Commitment,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';

import { decodeString } from '@switchboard-xyz/common';
import {
  asV0Tx,
  CrossbarClient,
  PullFeed,
  Queue,
  SB_ON_DEMAND_PID,
} from '@switchboard-xyz/on-demand';
import fs from 'fs';
import {
  Program as Anchor30Program,
  AnchorProvider,
  Wallet,
} from 'switchboard-anchor';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;

async function setupAnchor() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(USER_KEYPAIR!, {
          encoding: 'utf-8',
        }),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);

  return { userProvider, connection, user };
}

async function setupSwitchboard(userProvider: AnchorProvider) {
  const idl = await Anchor30Program.fetchIdl(SB_ON_DEMAND_PID, userProvider);
  const sbOnDemandProgram = new Anchor30Program(idl!, userProvider);
  let queue = new PublicKey('A43DyUGA7s8eXPxqEjJY6EBu1KKbNgfxF8h17VAHn13w');
  if (CLUSTER == 'devnet') {
    queue = new PublicKey('FfD96yeXs4cxZshoPPSKhSPgVQxLAJUT3gefgh84m1Di');
  }
  const crossbarClient = new CrossbarClient(
    'https://crossbar.switchboard.xyz',
    true,
  );
  return { sbOnDemandProgram, crossbarClient, queue };
}

(async function main(): Promise<void> {
  const { userProvider, connection, user } = await setupAnchor();

  const { sbOnDemandProgram, crossbarClient, queue } = await setupSwitchboard(
    userProvider,
  );

  const queueAccount = new Queue(sbOnDemandProgram, queue);
  try {
    await queueAccount.loadData();
  } catch (err) {
    console.error('Queue not found, ensure you are using devnet in your env');
    return;
  }

  const txOpts = {
    commitment: 'processed' as Commitment,
    skipPreflight: true,
    maxRetries: 0,
  };

  // TODO @Adrian
  const conf = {
    name: 'BTC Price Feed', // the feed name (max 32 bytes)
    queue, // the queue of oracles to bind to
    maxVariance: 1.0, // allow 1% variance between submissions and jobs
    minResponses: 1, // minimum number of responses of jobs to allow
    numSignatures: 3, // number of signatures to fetch per update
    minSampleSize: 1, // minimum number of responses to sample
    maxStaleness: 60, // maximum staleness of responses in seconds to sample
  };

  console.log('Initializing new data feed');
  // Generate the feed keypair
  const [pullFeed, feedKp] = PullFeed.generate(sbOnDemandProgram);
  const jobs = [
    // TODO @Adrian
    // source https://github.com/switchboard-xyz/sb-on-demand-examples/blob/main/sb-on-demand-feeds/scripts/utils.ts#L23
    // buildPythnetJob(
    //   'e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43',
    // ),
    // buildCoinbaseJob('BTC-USD'),
  ];
  const decodedFeedHash = await crossbarClient
    .store(queue.toBase58(), jobs)
    .then((resp) => decodeString(resp.feedHash));
  console.log('Feed hash:', decodedFeedHash);

  const tx = await asV0Tx({
    connection: sbOnDemandProgram.provider.connection,
    ixs: [await pullFeed.initIx({ ...conf, feedHash: decodedFeedHash! })],
    payer: user.publicKey,
    signers: [user, feedKp],
    computeUnitPrice: 75_000,
    computeUnitLimitMultiple: 1.3,
  });
  console.log('Sending initialize transaction');
  const sim = await connection.simulateTransaction(tx, txOpts);
  const sig = await connection.sendTransaction(tx, txOpts);
  console.log(`Feed ${feedKp.publicKey} initialized: ${sig}`);
})();
