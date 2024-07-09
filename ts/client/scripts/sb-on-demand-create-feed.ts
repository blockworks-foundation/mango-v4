import {
  LISTING_PRESETS,
  LISTING_PRESETS_KEY,
  tierSwitchboardSettings,
  tierToSwitchboardJobSwapValue,
} from '@blockworks-foundation/mango-v4-settings/lib/helpers/listingTools';
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
import { struct, u8, publicKey, u64, option } from '@raydium-io/raydium-sdk';
import * as toml from '@iarna/toml';
import { toNative } from '../src/utils';

// Configuration
const TIER: LISTING_PRESETS_KEY = 'asset_250';
const TOKEN_MINT = 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN';

// Tier based variables
const swapValue = tierToSwitchboardJobSwapValue[TIER];
const settingFromLib = tierSwitchboardSettings[TIER];
const maxVariance = LISTING_PRESETS[TIER].oracleConfFilter * 100;
const minResponses = settingFromLib!.minRequiredOracleResults;
const numSignatures = settingFromLib!.minRequiredOracleResults + 1;
const minSampleSize = settingFromLib!.minRequiredOracleResults;
const maxStaleness =
  LISTING_PRESETS[TIER].maxStalenessSlots === -1
    ? 10000
    : LISTING_PRESETS[TIER].maxStalenessSlots;

// Constants
const JUPITER_PRICE_API_MAINNET = 'https://price.jup.ag/v4/';
const JUPITER_TOKEN_API_MAINNET = 'https://token.jup.ag/all';
const WRAPPED_SOL_MINT = 'So11111111111111111111111111111111111111112';
const PYTH_SOL_ORACLE = 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG';
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const PYTH_USDC_ORACLE = 'Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD';
const SWITCHBOARD_USDC_ORACLE = 'FwYfsmj5x8YZXtQBNo2Cz8TE7WRCMFqA6UTffK4xQKMH';
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

async function getTokenPrice(mint: string): Promise<number> {
  const priceInfo = await (
    await fetch(`${JUPITER_PRICE_API_MAINNET}price?ids=${mint}`)
  ).json();
  //Note: if listing asset that don't have price on jupiter remember to edit this 0 to real price
  //in case of using 0 openbook market can be wrongly configured ignore if openbook market is existing
  const price = priceInfo.data[mint]?.price || 0;
  if (!price) {
    console.log('Token price not found');
    throw 'Token price not found';
  }
  return price;
}

async function getTokenInfo(mint: string): Promise<Token | undefined> {
  const response = await fetch(JUPITER_TOKEN_API_MAINNET);
  const data: Token[] = await response.json();
  const tokenInfo = data.find((x) => x.address === mint);
  if (!tokenInfo) {
    console.log('Token info not found');
    throw 'Token info not found';
  }
  return data.find((x) => x.address === mint);
}

async function getPool(mint: string): Promise<
  | {
      pool: string;
      poolSource: 'raydium' | 'orca';
      isSolPool: boolean;
      isReveredSolPool: boolean;
    }
  | undefined
> {
  const dex = await fetch(
    `https://api.dexscreener.com/latest/dex/search?q=${mint}`,
  );
  const resp = await dex.json();

  if (!resp?.pairs?.length) {
    return;
  }

  const pairs = resp.pairs.filter(
    (x) => x.dexId.includes('raydium') || x.dexId.includes('orca'),
  );

  const bestUsdcPool = pairs.find(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (x: any) => x.quoteToken.address === USDC_MINT,
  );

  const bestSolPool = pairs.find(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (x: any) => x.quoteToken.address === WRAPPED_SOL_MINT,
  );

  const bestReversedSolPool = pairs.find(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (x: any) => x.baseToken.address === WRAPPED_SOL_MINT,
  );

  if (bestUsdcPool) {
    return {
      pool: bestUsdcPool.pairAddress,
      poolSource: bestUsdcPool.dexId.includes('raydium') ? 'raydium' : 'orca',
      isSolPool: false,
      isReveredSolPool: false,
    };
  }

  if (bestSolPool) {
    return {
      pool: bestSolPool.pairAddress,
      poolSource: bestSolPool.dexId.includes('raydium') ? 'raydium' : 'orca',
      isSolPool: true,
      isReveredSolPool: false,
    };
  }

  if (bestSolPool) {
    return {
      pool: bestReversedSolPool.pairAddress,
      poolSource: bestReversedSolPool.dexId.includes('raydium')
        ? 'raydium'
        : 'orca',
      isSolPool: true,
      isReveredSolPool: true,
    };
  }

  console.log('No orca or raydium pool found');
  throw 'No orca or raydium pool found';
}

const getLstStakePool = async (
  connection: Connection,
  mint: string,
): Promise<string> => {
  try {
    let poolAddress = '';
    let addresses: string[] = [];
    try {
      const tomlFile = await fetch(
        `https://raw.githubusercontent.com/${'igneous-labs'}/${'sanctum-lst-list'}/master/sanctum-lst-list.toml`,
      );

      const tomlText = await tomlFile.text();
      const tomlData = toml.parse(tomlText) as unknown as {
        sanctum_lst_list: { pool: { pool: string } }[];
      };
      addresses = [
        ...tomlData.sanctum_lst_list
          .map((x) => tryGetPubKey(x.pool.pool)?.toBase58())
          .filter((x) => x),
      ] as string[];
    } catch (e) {
      console.log(e);
    }

    //remove duplicates
    const possibleStakePoolsAddresses = [...new Set(addresses)].map(
      (x) => new PublicKey(x),
    );

    const accounts = await connection.getMultipleAccountsInfo(
      possibleStakePoolsAddresses,
    );
    for (const idx in accounts) {
      try {
        const acc = accounts[idx];
        const stakeAddressPk = possibleStakePoolsAddresses[idx];
        if (acc?.data) {
          const decoded = StakePoolLayout.decode(acc?.data);
          if (decoded.poolMint.toBase58() === mint && stakeAddressPk) {
            poolAddress = stakeAddressPk?.toBase58();
            break;
          }
        }
        // eslint-disable-next-line no-empty
      } catch (e) {}
    }

    return poolAddress;
  } catch (e) {
    console.log(e);
    return '';
  }
};

const LSTExactIn = (
  inMint: string,
  nativeInAmount: string,
  stakePoolAddress: string,
): string => {
  const template = `tasks:
        - conditionalTask:
            attempt:
            - httpTask:
                      url: https://api.sanctum.so/v1/swap/quote?input=${inMint}&outputLstMint=So11111111111111111111111111111111111111112&amount=${nativeInAmount}&mode=ExactIn
            - jsonParseTask:
                      path: $.outAmount
            - divideTask:
                     scalar: ${nativeInAmount}
            onFailure:
            - splStakePoolTask:
                pubkey: ${stakePoolAddress}
            - cacheTask:
                cacheItems:
                  - variableName: poolTokenSupply
                    job:
                      tasks:
                        - jsonParseTask:
                            path: $.uiPoolTokenSupply
                            aggregationMethod: NONE
                  - variableName: totalStakeLamports
                    job:
                      tasks:
                        - jsonParseTask:
                            path: $.uiTotalLamports
                            aggregationMethod: NONE
            - valueTask:
                big: \${totalStakeLamports}
            - divideTask:
                big: \${poolTokenSupply}
        - multiplyTask:
                  job:
                    tasks:
                      - oracleTask:
                          pythAddress: H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG
                          pythAllowedConfidenceInterval: 10`;
  return template;
};

const LSTExactOut = (
  inMint: string,
  nativeOutSolAmount: string,
  stakePoolAddress: string,
): string => {
  const template = `tasks:
      - conditionalTask:
          attempt:
            - cacheTask:
                cacheItems:
                  - variableName: QTY
                    job:
                      tasks:
                        - httpTask:
                            url: https://api.sanctum.so/v1/swap/quote?input=${inMint}&outputLstMint=So11111111111111111111111111111111111111112&amount=${nativeOutSolAmount}&mode=ExactOut
                        - jsonParseTask:
                                  path: $.inAmount
            - httpTask:
                 url: https://api.sanctum.so/v1/swap/quote?input=${inMint}&outputLstMint=So11111111111111111111111111111111111111112&amount=\${QTY}&mode=ExactIn
            - jsonParseTask:
                path: $.outAmount
            - divideTask:
                big: \${QTY}
          onFailure:
              - splStakePoolTask:
                  pubkey: ${stakePoolAddress}
              - cacheTask:
                  cacheItems:
                    - variableName: poolTokenSupply
                      job:
                        tasks:
                          - jsonParseTask:
                              path: $.uiPoolTokenSupply
                              aggregationMethod: NONE
                    - variableName: totalStakeLamports
                      job:
                        tasks:
                          - jsonParseTask:
                              path: $.uiTotalLamports
                              aggregationMethod: NONE
              - valueTask:
                  big: \${totalStakeLamports}
              - divideTask:
                  big: \${poolTokenSupply}
      - multiplyTask:
            job:
              tasks:
                - oracleTask:
                    pythAddress: H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG
                    pythAllowedConfidenceInterval: 10`;
  return template;
};

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
  const [
    { sbOnDemandProgram, crossbarClient, queue },
    poolInfo,
    price,
    tokeninfo,
    lstPool,
  ] = await Promise.all([
    setupSwitchboard(userProvider),
    getPool(TOKEN_MINT),
    getTokenPrice(TOKEN_MINT),
    getTokenInfo(TOKEN_MINT),
    getLstStakePool(connection, TOKEN_MINT),
  ]);

  const FALLBACK_POOL_NAME: 'orcaPoolAddress' | 'raydiumPoolAddress' = `${
    poolInfo?.poolSource || 'raydium'
  }PoolAddress`;
  const FALLBACK_POOL = poolInfo?.pool;
  const TOKEN_SYMBOL = tokeninfo!.symbol.toUpperCase();

  const queueAccount = new Queue(sbOnDemandProgram, queue);
  try {
    await queueAccount.loadData();
  } catch (err) {
    console.error('Queue not found, ensure you are using devnet in your env');
    return;
  }

  let onFailureTaskDesc: { [key: string]: any }[];
  if (!poolInfo?.isReveredSolPool) {
    onFailureTaskDesc = [
      {
        lpExchangeRateTask: {
          [FALLBACK_POOL_NAME]: FALLBACK_POOL,
        },
      },
    ];
    if (poolInfo?.isSolPool) {
      onFailureTaskDesc.push({
        multiplyTask: {
          job: {
            tasks: [
              {
                oracleTask: {
                  pythAddress: PYTH_SOL_ORACLE,
                  pythAllowedConfidenceInterval: 10,
                },
              },
            ],
          },
        },
      });
    }
  } else {
    onFailureTaskDesc = [
      {
        valueTask: {
          big: 1,
        },
      },
      {
        divideTask: {
          job: {
            tasks: [
              {
                lpExchangeRateTask: {
                  [FALLBACK_POOL_NAME]: FALLBACK_POOL,
                },
              },
            ],
          },
        },
      },
    ];
    if (poolInfo.isSolPool) {
      onFailureTaskDesc.push({
        multiplyTask: {
          job: {
            tasks: [
              {
                oracleTask: {
                  pythAddress: PYTH_SOL_ORACLE,
                  pythAllowedConfidenceInterval: 10,
                },
              },
            ],
          },
        },
      });
    }
  }

  const txOpts = {
    commitment: 'finalized' as Commitment,
    skipPreflight: true,
    maxRetries: 0,
  };

  const conf = {
    name: `${TOKEN_SYMBOL}/USD`, // the feed name (max 32 bytes)
    queue, // the queue of oracles to bind to
    maxVariance: maxVariance!, // allow 1% variance between submissions and jobs
    minResponses: minResponses!, // minimum number of responses of jobs to allow
    numSignatures: numSignatures!, // number of signatures to fetch per update
    minSampleSize: minSampleSize!, // minimum number of responses to sample
    maxStaleness: maxStaleness!, // maximum staleness of responses in seconds to sample
  };

  console.log('Initializing new data feed');
  // Generate the feed keypair
  const [pullFeed, feedKp] = PullFeed.generate(sbOnDemandProgram);
  const jobs = [
    lstPool
      ? OracleJob.fromYaml(
          LSTExactIn(
            TOKEN_MINT,
            toNative(
              Math.ceil(Number(swapValue) / price),
              tokeninfo!.decimals,
            ).toString(),
            lstPool,
          ),
        )
      : OracleJob.fromObject({
          tasks: [
            {
              conditionalTask: {
                attempt: [
                  {
                    valueTask: {
                      big: swapValue,
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
                              baseAmountString: swapValue,
                            },
                          },
                        ],
                      },
                    },
                  },
                ],
                onFailure: onFailureTaskDesc,
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
                              pythAddress: PYTH_USDC_ORACLE,
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
                              switchboardAddress: SWITCHBOARD_USDC_ORACLE,
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
    lstPool
      ? OracleJob.fromYaml(
          LSTExactOut(
            TOKEN_MINT,
            toNative(
              Math.ceil(Number(swapValue) / price),
              tokeninfo!.decimals,
            ).toString(),
            lstPool,
          ),
        )
      : OracleJob.fromObject({
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
                                  baseAmountString: swapValue,
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
                onFailure: onFailureTaskDesc,
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
                              pythAddress: PYTH_USDC_ORACLE,
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
                              switchboardAddress: SWITCHBOARD_USDC_ORACLE,
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

export type Token = {
  address: string;
  chainId: number;
  decimals: number;
  name: string;
  symbol: string;
  logoURI: string;
  extensions: {
    coingeckoId?: string;
  };
  tags: string[];
};

const feeFields = [u64('denominator'), u64('numerator')];
const StakePoolLayout = struct([
  u8('accountType'),
  publicKey('manager'),
  publicKey('staker'),
  publicKey('stakeDepositAuthority'),
  u8('stakeWithdrawBumpSeed'),
  publicKey('validatorList'),
  publicKey('reserveStake'),
  publicKey('poolMint'),
  publicKey('managerFeeAccount'),
  publicKey('tokenProgramId'),
  u64('totalLamports'),
  u64('poolTokenSupply'),
  u64('lastUpdateEpoch'),
  struct(
    [u64('unixTimestamp'), u64('epoch'), publicKey('custodian')],
    'lockup',
  ),
  struct(feeFields, 'epochFee'),
  option(struct(feeFields), 'nextEpochFee'),
  option(publicKey(), 'preferredDepositValidatorVoteAddress'),
  option(publicKey(), 'preferredWithdrawValidatorVoteAddress'),
  struct(feeFields, 'stakeDepositFee'),
  struct(feeFields, 'stakeWithdrawalFee'),
  option(struct(feeFields), 'nextStakeWithdrawalFee'),
  u8('stakeReferralFee'),
  option(publicKey(), 'solDepositAuthority'),
  struct(feeFields, 'solDepositFee'),
  u8('solReferralFee'),
  option(publicKey(), 'solWithdrawAuthority'),
  struct(feeFields, 'solWithdrawalFee'),
  option(struct(feeFields), 'nextSolWithdrawalFee'),
  u64('lastEpochPoolTokenSupply'),
  u64('lastEpochTotalLamports'),
]);

const tryGetPubKey = (pubkey: string | string[]) => {
  try {
    return new PublicKey(pubkey);
  } catch (e) {
    return null;
  }
};
