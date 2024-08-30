import {
  AccountInfo,
  Cluster,
  Commitment,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  CrossbarClient,
  ON_DEMAND_MAINNET_PID,
  PullFeed,
  Queue,
  RecentSlotHashes,
} from '@switchboard-xyz/on-demand';
import fs from 'fs';
import chunk from 'lodash/chunk';
import shuffle from 'lodash/shuffle';
import uniqWith from 'lodash/uniqWith';
import zipWith from 'lodash/zipWith';
import { Program as Anchor30Program, BN, Idl } from 'switchboard-anchor';

import { SequenceType } from '@blockworks-foundation/mangolana/lib/globalTypes';
import { sendSignAndConfirmTransactions } from '@blockworks-foundation/mangolana/lib/transactions';
import { BorshAccountsCoder } from '@coral-xyz/anchor';
import { AnchorProvider, Wallet } from 'switchboard-anchor';
import { TokenIndex } from '../src/accounts/bank';
import { Group } from '../src/accounts/group';
import {
  isOracleStaleOrUnconfident,
  parseSwitchboardOracle,
} from '../src/accounts/oracle';
import { PerpMarketIndex } from '../src/accounts/perp';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID, MANGO_V4_MAIN_GROUP } from '../src/constants';
import { createComputeBudgetIx } from '../src/utils/rpc';
import { manageFeeWebSocket } from './manageFeeWs';
import {
  getOraclesForMangoGroup,
  OraclesFromMangoGroupInterface,
} from './sb-on-demand-crank-utils';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const CLUSTER_URL_2 = process.env.MB_CLUSTER_URL_2;
const LITE_RPC_URL = process.env.LITE_RPC_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP = process.env.GROUP_OVERRIDE || MANGO_V4_MAIN_GROUP.toBase58();
const SLEEP_MS = Number(process.env.SLEEP_MS) || 20_000;

console.log(
  `[start] config: sleep ${SLEEP_MS}ms, cluster ${CLUSTER_URL}, cluster2 ${CLUSTER_URL_2}, liteRpcUrl ${LITE_RPC_URL}`,
);

let lamportsPerCu: number | null = null;
try {
  const wsUrl = new URL(
    process.env.LITE_RPC_URL!.replace('https', 'wss'),
  ).toString();

  manageFeeWebSocket(wsUrl, 10, (mean) => {
    lamportsPerCu = mean;
  });
} catch (error) {
  console.error('[start]', error);
}

interface OracleMetaInterface {
  oracle: {
    oraclePk: PublicKey;
    name: string;
    fallbackForOracle: PublicKey | undefined;
    tokenIndex: TokenIndex | undefined;
    perpMarketIndex: PerpMarketIndex | undefined;
    isOracleStaleOrUnconfident: boolean;
  };
  ai: AccountInfo<Buffer> | null;
  decodedPullFeed: any;
  parsedConfigs: {
    queue: any;
    maxVariance: number;
    minResponses: any;
    feedHash: any;
    ipfsHash: any;
  };
  jobs: any[];
  gatewayUrl: string;
}

/// refresh mango group to detect new oracles added through governance
/// without a restart within 1 minute, result object will be dynamically
/// updated
async function setupBackgroundRefresh(
  client: MangoClient,
  group: Group,
  sbOnDemandProgram: Anchor30Program<Idl>,
  crossbarClient: CrossbarClient,
): Promise<{ oracles: OracleMetaInterface[] }> {
  // note: group was already reloaded before
  const oracles = await prepareCandidateOracles(
    client,
    group,
    sbOnDemandProgram,
    crossbarClient,
  );

  const result = { oracles };

  const GROUP_REFRESH_INTERVAL = 60_000;
  const refreshGroup = async function (): Promise<void> {
    try {
      await group.reloadAll(client);
      result.oracles = await prepareCandidateOracles(
        client,
        group,
        sbOnDemandProgram,
        crossbarClient,
      );
    } catch (e) {
      console.error('[group]', e);
    }
    setTimeout(refreshGroup, GROUP_REFRESH_INTERVAL);
  };

  setTimeout(refreshGroup, GROUP_REFRESH_INTERVAL);

  return result;
}

(async function main(): Promise<never> {
  const { group, client, connection, user } = await setupMango();

  const { sbOnDemandProgram, crossbarClient } = await setupSwitchboard(client);

  const refresh = await setupBackgroundRefresh(
    client,
    group,
    sbOnDemandProgram,
    crossbarClient,
  );

  while (true) {
    try {
      // pull a fresh reference to the oracles from the background refresher
      const { oracles } = refresh;

      const startedAt = Date.now();
      const [block, slot] = await Promise.all([
        // use finalized blockhash for faster timeouts on transactions
        client.connection.getLatestBlockhash('finalized'),
        // use processed slot for accurate staleness measurement
        client.connection.getSlot('processed'),
      ]);

      await updateFilteredOraclesAis(
        client,
        sbOnDemandProgram,
        group,
        slot,
        oracles,
      );

      const onlyEssentialOracles = oracles.filter(
        (o) => o.oracle.isOracleStaleOrUnconfident,
      );

      const aisUpdatedAt = Date.now();

      const staleOracles = await filterForStaleOracles(
        onlyEssentialOracles,
        client,
        slot,
      );

      const staleFilteredAt = Date.now();

      const crossBarSims = await Promise.all(
        onlyEssentialOracles.map((o) =>
          crossbarClient.simulateFeeds([
            new Buffer(o.parsedConfigs.feedHash).toString('hex'),
          ]),
        ),
      );

      const simulatedAt = Date.now();

      const varianceThresholdCrossedOracles =
        await filterForVarianceThresholdOracles(
          onlyEssentialOracles,
          client,
          crossBarSims,
        );

      const varianceFilteredAt = Date.now();

      const oraclesToCrank: OracleMetaInterface[] = uniqWith(
        [...staleOracles, ...varianceThresholdCrossedOracles],
        function (a, b) {
          return a.oracle.oraclePk.equals(b.oracle.oraclePk);
        },
      );

      console.log(
        `[main] round candidates | Stale: ${staleOracles
          .map((o) => o.oracle.name)
          .join(', ')} | Variance: ${varianceThresholdCrossedOracles
          .map((o) => o.oracle.name)
          .join(', ')}`,
      );

      // todo use chunk
      // todo use luts

      // const [pullIxs, luts] = await PullFeed.fetchUpdateManyIx(
      //   sbOnDemandProgram as any,
      //   {
      //     feeds: oraclesToCrank.map((o) => new PublicKey(o.oracle.oraclePk)),
      //     numSignatures: 3,
      //   },
      // );

      const recentSlothashes = await RecentSlotHashes.fetchLatestNSlothashes(
        connection as any,
        30,
      );
      const pullIxs = (
        await Promise.all(
          oraclesToCrank.map(async (oracle) => {
            const pullIx = await preparePullIx(
              sbOnDemandProgram,
              oracle,
              recentSlothashes,
            );
            return pullIx !== undefined ? pullIx : null;
          }),
        )
      ).filter((pullIx) => pullIx !== null);

      const ixPreparedAt = Date.now();

      const ixsChunks = chunk(shuffle(pullIxs), 2, false);
      const lamportsPerCu_ = Math.min(
        Math.max(lamportsPerCu ?? 150_000, 150_000),
        500_000,
      );

      // dont await, fire and forget
      sendSignAndConfirmTransactions({
        connection,
        wallet: new Wallet(user),
        backupConnections: [
          ...(CLUSTER_URL_2 ? [new Connection(LITE_RPC_URL!, 'recent')] : []),
          ...(CLUSTER_URL_2 ? [new Connection(CLUSTER_URL_2!, 'recent')] : []),
        ],
        // fail rather quickly and retry submission from scratch
        // timeout using finalized to stay below switchboard oracle staleness limit
        timeoutStrategy: { block, startBlockCheckAfterSecs: 20 },
        transactionInstructions: ixsChunks.map((txChunk) => ({
          instructionsSet: [
            {
              signers: [],
              transactionInstruction: createComputeBudgetIx(lamportsPerCu_),
            },
            ...txChunk.map((tx) => ({
              signers: [],
              transactionInstruction: tx,
              // TODO disable alts for now
              // alts: SBOD_ORACLE_LUTS.map(x=>new PublicKey(x)),
            })),
          ],
          sequenceType: SequenceType.Parallel,
        })),
        config: {
          maxTxesInBatch: 10,
          autoRetry: false,
          logFlowInfo: false,
          // TODO disable alts for now
          // useVersionedTransactions: true,
        },
        callbacks: {
          afterEveryTxSend: function (data) {
            const sentAt = Date.now();
            const total = (sentAt - startedAt) / 1000;
            const aiUpdate = (aisUpdatedAt - startedAt) / 1000;
            const staleFilter = (staleFilteredAt - aisUpdatedAt) / 1000;
            const simulate = (simulatedAt - staleFilteredAt) / 1000;
            const varianceFilter = (varianceFilteredAt - simulatedAt) / 1000;
            const ixPrepare = (ixPreparedAt - varianceFilteredAt) / 1000;
            const timing = {
              aiUpdate,
              staleFilter,
              simulate,
              varianceFilter,
              ixPrepare,
            };

            console.log(
              `[tx send] https://solscan.io/tx/${data['txid']}, in ${total}s, lamportsPerCu_ ${lamportsPerCu_}, lamportsPerCu ${lamportsPerCu}, timiming ${JSON.stringify(timing)}`,
            );
          },
          onError: function (e, notProcessedTransactions) {
            console.error(
              `[tx send] ${notProcessedTransactions.length} error(s) after ${(Date.now() - ixPreparedAt) / 1000}s ${JSON.stringify(e)}`,
            );
          },
        },
      }).catch((reason) =>
        console.error(
          `[tx send] promise rejected after ${(Date.now() - ixPreparedAt) / 1000}s ${JSON.stringify(reason)}`,
        ),
      );

      await new Promise((r) => setTimeout(r, SLEEP_MS));
    } catch (error) {
      console.error('[main]', error);
    }
  }
})();

/**
 * prepares the instruction to update an individual oracle using the cached data on oracle
 */
async function preparePullIx(
  sbOnDemandProgram,
  oracle: OracleMetaInterface,
  recentSlothashes?: Array<[BN, string]>,
): Promise<TransactionInstruction | undefined | null> {
  const pullFeed = new PullFeed(
    sbOnDemandProgram as any,
    new PublicKey(oracle.oracle.oraclePk),
  );

  const conf = {
    numSignatures: oracle.parsedConfigs.minResponses,
    feed: oracle.oracle.oraclePk,
    feedConfigs: oracle.parsedConfigs,
    gateway: oracle.gatewayUrl,
  };
  // TODO use fetchUpdateMany

  try {
    const [pullIx] = await pullFeed.fetchUpdateIx(conf, recentSlothashes);
    return pullIx;
  } catch (error) {
    console.log(`[preparePullIx] ${oracle.oracle.name} error ${error}`);
    return null;
  }
}

async function filterForVarianceThresholdOracles(
  filteredOracles: OracleMetaInterface[],
  client: MangoClient,
  crossBarSims,
): Promise<OracleMetaInterface[]> {
  const varianceThresholdCrossedOracles = new Array<OracleMetaInterface>();
  for (const [index, item] of filteredOracles.entries()) {
    const res = await parseSwitchboardOracle(
      item.oracle.oraclePk,
      item.ai!,
      client.connection,
    );
    // console.log(`${item.oracle.name} ${JSON.stringify(res)}`);

    const crossBarSim = crossBarSims[index];

    const simPrice =
      crossBarSim[0].results.reduce((a, b) => a + b, 0) /
      crossBarSim[0].results.length;

    const changePct = (Math.abs(res.price - simPrice) * 100) / res.price;
    if (changePct > item.decodedPullFeed.maxVariance / 1000000000) {
      console.log(
        `[filter variance] ${item.oracle.name}, candidate, ${
          item.decodedPullFeed.maxVariance / 1000000000
        }, ${simPrice}, ${res.price}, ${changePct}`,
      );
      varianceThresholdCrossedOracles.push(item);
    } else {
      console.log(
        `[filter variance] ${item.oracle.name}, non-candidate, ${
          item.decodedPullFeed.maxVariance / 1000000000
        }, ${simPrice}, ${res.price}, ${changePct}`,
      );
    }
  }
  return varianceThresholdCrossedOracles;
}

async function filterForStaleOracles(
  filteredOracles: OracleMetaInterface[],
  client: MangoClient,
  slot: number,
): Promise<OracleMetaInterface[]> {
  const staleOracles = new Array<OracleMetaInterface>();
  for (const item of filteredOracles) {
    const res = await parseSwitchboardOracle(
      item.oracle.oraclePk,
      item.ai!,
      client.connection,
    );

    const diff = slot - res.lastUpdatedSlot;
    if (
      // maxStaleness will usually be 250 (=100s)
      // one iteration takes 10s, retry is every 20s
      // this allows for 2 retries until the oracle becomes stale
      diff >
      item.decodedPullFeed.maxStaleness * 0.3
    ) {
      console.log(
        `[filter stale] ${item.oracle.name}, candidate, ${item.decodedPullFeed.maxStaleness}, ${slot}, ${res.lastUpdatedSlot}, ${diff}`,
      );
      staleOracles.push(item);
    } else {
      console.log(
        `[filter stale] ${item.oracle.name}, non-candidate, ${item.decodedPullFeed.maxStaleness}, ${slot}, ${res.lastUpdatedSlot}, ${diff}`,
      );
    }
  }
  return staleOracles;
}

/**
 * fetch all on-demand oracles used on mango group and parse their configuration
 */
async function prepareCandidateOracles(
  client: MangoClient,
  group: Group,
  sbOnDemandProgram: Anchor30Program<Idl>,
  crossbarClient: CrossbarClient,
): Promise<OracleMetaInterface[]> {
  // collect
  const oracles = getOraclesForMangoGroup(group);
  oracles.push(...extendOraclesManually(CLUSTER));

  // load all oracle account infos
  const ais = (
    await Promise.all(
      chunk(
        oracles.map((item) => item.oraclePk),
        50,
        false,
      ).map(
        async (chunk) =>
          await client.program.provider.connection.getMultipleAccountsInfo(
            chunk,
          ),
      ),
    )
  ).flat();

  // ensure rpc response is correct
  for (const [idx, ai] of ais.entries()) {
    if (ai == null || ai.data == null) {
      throw new Error(
        `AI returned null for ${oracles[idx].name} ${oracles[idx].oraclePk}!`,
      );
    }
  }
  if (ais.length != oracles.length) {
    throw new Error(
      `Expected ${oracles.length}, but gMA returned ${ais.length}!`,
    );
  }

  // combine account info
  const sbodOracles = oracles
    .map((o, i) => {
      return { oracle: o, ai: ais[i] };
    })
    .filter((item) => item.ai?.owner.equals(ON_DEMAND_MAINNET_PID));

  // parse account info data
  const parsedOracles = sbodOracles.map((item) => {
    const d = sbOnDemandProgram.coder.accounts.decode(
      'pullFeedAccountData',
      item.ai!.data,
    );
    return {
      decodedPullFeed: d,
      parsedConfigs: {
        queue: d.queue,
        maxVariance: d.maxVariance / 1e9,
        minResponses: d.minResponses,
        feedHash: d.feedHash,
        ipfsHash: d.ipfsHash,
      },
    };
  });

  const jobs = await Promise.all(
    parsedOracles.map((o) =>
      crossbarClient
        .fetch(Buffer.from(o.parsedConfigs.feedHash).toString('hex'))
        .then((r) => r.jobs),
    ),
  );

  const gateways = await Promise.all(
    parsedOracles.map((o) =>
      new Queue(sbOnDemandProgram, o.parsedConfigs.queue).fetchAllGateways(),
    ),
  );

  // assemble all data together
  return sbodOracles.map((o, i) => ({
    ...o,
    ...parsedOracles[i],
    jobs: jobs[i],
    gatewayUrl: gateways[i][0].gatewayUrl,
  }));
}

function extendOraclesManually(
  cluster: Cluster,
): OraclesFromMangoGroupInterface[] {
  if (cluster == 'devnet') {
    return [
      {
        oraclePk: new PublicKey('EtbG8PSDCyCSmDH8RE4Nf2qTV9d6P6zShzHY2XWvjFJf'),
        name: 'BTC/USD',
        fallbackForOracle: undefined,
        tokenIndex: undefined,
        perpMarketIndex: undefined,
        isOracleStaleOrUnconfident: false,
      },
    ];
  }
  return [
    // These are now set on the group, and fetched from there
    // ['JSOL/USD', 'Dnn9fKeB3rA2bor6Fys7FBPqXneAK8brxNfsBfZ32939'],
    // ['compassSOL/USD', 'GzBpasKMSTLkytXpyo6NesDGpe2mLjPSovECWsebQpu5'],
    // ['dualSOL/USD', 'D6UqFgtVC1yADBxw2EZFmUCTNuoqFoUXD3NW4NqRn8v3'],
    // ['hubSOL/USD', '7LRVXc8zdPpzXNdknU2kRTYt7BizYs7BaM6Ft2zv8E4h'],
    // ['hubSOL/USD', '137fd2LnDEPVAALhPFjRyvh2MD9DxSHPFaod7a5tmMox'],
    // ['digitSOL/USD', '7skmP8qLf8KKJ61cpPiw91GXYfoGvGWekzSDQ78T3z1f'],
    // ['mangoSOL/USD', '7pD4Y1hCsU4M6rfoJvL8fAmmrB2LwrJYxvWz4S6Cc24T'],
  ].map((item) => {
    return {
      oraclePk: new PublicKey(item[1]),
      name: item[0],
      fallbackForOracle: undefined,
      tokenIndex: undefined,
      perpMarketIndex: undefined,
      isOracleStaleOrUnconfident: false,
    };
  });
}

async function setupMango(): Promise<{
  group: Group;
  client: MangoClient;
  connection: Connection;
  user: Keypair;
  userProvider: AnchorProvider;
}> {
  // the connection needs to be set to confirmed so that we never
  // submit an oracle update with a processed -> forked away slot hash
  const options = { commitment: 'confirmed' as Commitment };
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'api',
    },
  );

  const group = await client.getGroup(new PublicKey(GROUP));
  await group.reloadAll(client);
  return { group, client, connection, user, userProvider };
}

async function setupSwitchboard(client: MangoClient): Promise<{
  sbOnDemandProgram: Anchor30Program<Idl>;
  crossbarClient: CrossbarClient;
  queue: PublicKey;
}> {
  const idl = await Anchor30Program.fetchIdl(
    ON_DEMAND_MAINNET_PID,
    client.program.provider,
  );
  const sbOnDemandProgram = new Anchor30Program(idl!, client.program.provider);
  let queue = new PublicKey('A43DyUGA7s8eXPxqEjJY6EBu1KKbNgfxF8h17VAHn13w');
  if (CLUSTER == 'devnet') {
    queue = new PublicKey('FfD96yeXs4cxZshoPPSKhSPgVQxLAJUT3gefgh84m1Di');
  }
  const crossbarClient = new CrossbarClient(
    'https://crossbar.switchboard.xyz',
    false,
  );
  return { sbOnDemandProgram, crossbarClient, queue };
}

/**
 * reloads the account states for each oracle passed through the provided connection
 */
async function updateFilteredOraclesAis(
  client: MangoClient,
  sbOnDemandProgram: Anchor30Program<Idl>,
  group: Group,
  nowSlot: number,
  filteredOracles: OracleMetaInterface[],
): Promise<void> {
  const ais = (
    await Promise.all(
      chunk(
        filteredOracles.map((item) => item.oracle.oraclePk),
        50,
        false,
      ).map((chunk) =>
        client.program.provider.connection.getMultipleAccountsInfo(chunk),
      ),
    )
  ).flat();

  filteredOracles.forEach((fo, idx) => {
    fo.ai = ais[idx];

    const decodedPullFeed = sbOnDemandProgram.coder.accounts.decode(
      'pullFeedAccountData',
      fo.ai!.data,
    );
    fo.decodedPullFeed = decodedPullFeed;
  });

  // make a note iff a sbod oracle, is a fallback for another oracle, where the main oracle is stale or unconfident
  {
    // filter where sbod oracle is a fallback for another oracle
    const publicKeysWithIndices = filteredOracles
      .map((item, idx) => {
        return { publicKey: item.oracle.fallbackForOracle, idx: idx };
      })
      .filter((item) => item.publicKey !== undefined);
    const ais = (
      await Promise.all(
        chunk(
          publicKeysWithIndices.map((item) => item.publicKey),
          50,
          false,
        ).map((chunk_) =>
          client.program.provider.connection.getMultipleAccountsInfo(chunk_),
        ),
      )
    ).flat();

    const coder = new BorshAccountsCoder(client.program.idl);

    await Promise.all(
      zipWith(publicKeysWithIndices, ais, function (publicKeyWithIndex, ai) {
        return { publicKeyWithIndex, ai };
      }).map(async (item) => {
        // fetch main oracle, and check if it is stale or unconfident
        const filteredOracle = filteredOracles[item.publicKeyWithIndex.idx];
        let mintDecimals, maxStalenessSlots, confFilter;
        if (filteredOracle.oracle.tokenIndex !== undefined) {
          const bank = group.getFirstBankByTokenIndex(
            filteredOracle.oracle.tokenIndex,
          );
          mintDecimals = bank.mintDecimals;
          maxStalenessSlots = bank.oracleConfig.maxStalenessSlots;
          confFilter = bank.oracleConfig.confFilter;
        } else if (filteredOracle.oracle.perpMarketIndex !== undefined) {
          const pm = group.getPerpMarketByMarketIndex(
            filteredOracle.oracle.perpMarketIndex,
          );
          mintDecimals = pm.baseDecimals;
          maxStalenessSlots = pm.oracleConfig.maxStalenessSlots;
          confFilter = pm.oracleConfig.confFilter;
        } else {
          return;
        }
        const result = await Group.decodePriceFromOracleAi(
          group,
          coder,
          filteredOracle.oracle.oraclePk,
          item.ai,
          mintDecimals,
          client,
        );
        filteredOracle.oracle.isOracleStaleOrUnconfident =
          isOracleStaleOrUnconfident(
            nowSlot,
            maxStalenessSlots.toNumber() - 100, // mark as stale a bit early, so we can keep fallback prepared
            result.lastUpdatedSlot,
            result.deviation,
            confFilter,
            result.price,
            true,
            `${filteredOracle.oracle.name} fallback-main-oracle-check`,
          );
      }),
    );
  }
}
