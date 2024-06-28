import { LISTING_PRESETS } from '@blockworks-foundation/mango-v4-settings/lib/helpers/listingTools';
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
  OracleJob,
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
// basic configuration
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const SWAP_VALUE = '100';
const TOKEN_MINT = 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac';
const FALLBACK_POOL_NAME: 'orcaPoolAddress' | 'raydiumPoolAddress' =
  'raydiumPoolAddress';
const FALLBACK_POOL = '34tFULRrRwh4bMcBLPtJaNqqe5pVgGZACi5sR8Xz95KC';
const TOKEN_SYMBOL = 'MNGO';
// basic configuration

const pythUsdOracle = 'Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD';
const switchboardUsdDaoOracle = 'FwYfsmj5x8YZXtQBNo2Cz8TE7WRCMFqA6UTffK4xQKMH';
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
  const tier = Object.values(LISTING_PRESETS).find(
    (x) => x.preset_name === 'C',
  );

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
    commitment: 'finalized' as Commitment,
    skipPreflight: true,
    maxRetries: 0,
  };

  const conf = {
    name: `${TOKEN_SYMBOL}/USD`, // the feed name (max 32 bytes)
    queue, // the queue of oracles to bind to
    maxVariance: 10, // allow 1% variance between submissions and jobs
    minResponses: 2, // minimum number of responses of jobs to allow
    numSignatures: 3, // number of signatures to fetch per update
    minSampleSize: 2, // minimum number of responses to sample
    maxStaleness: tier!.maxStalenessSlots!, // maximum staleness of responses in seconds to sample
  };

  console.log('Initializing new data feed');
  // Generate the feed keypair
  const [pullFeed, feedKp] = PullFeed.generate(sbOnDemandProgram);
  const jobs = [
    OracleJob.fromObject({
      tasks: [
        {
          conditionalTask: {
            attempt: [
              {
                valueTask: {
                  big: SWAP_VALUE,
                },
              },
              {
                divideTask: {
                  job: {
                    tasks: [
                      {
                        jupiterSwapTask: {
                          inTokenAddress: USDC_MINT,
                          outTokenAddress: TOKEN_MINT,
                          baseAmountString: SWAP_VALUE,
                        },
                      },
                    ],
                  },
                },
              },
            ],
            onFailure: [
              {
                lpExchangeRateTask: {
                  [FALLBACK_POOL_NAME]: FALLBACK_POOL,
                },
              },
            ],
          },
        },
        {
          conditionalTask: {
            attempt: [
              {
                multiplyTask: {
                  job: {
                    tasks: [
                      {
                        oracleTask: {
                          pythAddress: pythUsdOracle,
                          pythAllowedConfidenceInterval: 10,
                        },
                      },
                    ],
                  },
                },
              },
            ],
            onFailure: [
              {
                multiplyTask: {
                  job: {
                    tasks: [
                      {
                        oracleTask: {
                          switchboardAddress: switchboardUsdDaoOracle,
                        },
                      },
                    ],
                  },
                },
              },
            ],
          },
        },
      ],
    }),
    OracleJob.fromObject({
      tasks: [
        {
          conditionalTask: {
            attempt: [
              {
                cacheTask: {
                  cacheItems: [
                    {
                      variableName: 'QTY',
                      job: {
                        tasks: [
                          {
                            jupiterSwapTask: {
                              inTokenAddress: USDC_MINT,
                              outTokenAddress: TOKEN_MINT,
                              baseAmountString: SWAP_VALUE,
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                jupiterSwapTask: {
                  inTokenAddress: TOKEN_MINT,
                  outTokenAddress: USDC_MINT,
                  baseAmountString: '${QTY}',
                },
              },
              {
                divideTask: {
                  big: '${QTY}',
                },
              },
            ],
            onFailure: [
              {
                lpExchangeRateTask: {
                  [FALLBACK_POOL_NAME]: FALLBACK_POOL,
                },
              },
            ],
          },
        },
        {
          conditionalTask: {
            attempt: [
              {
                multiplyTask: {
                  job: {
                    tasks: [
                      {
                        oracleTask: {
                          pythAddress: pythUsdOracle,
                          pythAllowedConfidenceInterval: 10,
                        },
                      },
                    ],
                  },
                },
              },
            ],
            onFailure: [
              {
                multiplyTask: {
                  job: {
                    tasks: [
                      {
                        oracleTask: {
                          switchboardAddress: switchboardUsdDaoOracle,
                        },
                      },
                    ],
                  },
                },
              },
            ],
          },
        },
      ],
    }),
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
