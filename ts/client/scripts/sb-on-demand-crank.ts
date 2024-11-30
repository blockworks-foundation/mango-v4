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
} from '@switchboard-xyz/on-demand';
import fs from 'fs';
import chunk from 'lodash/chunk';
import uniqWith from 'lodash/uniqWith';
import { Program as Anchor30Program, BN, Idl } from 'switchboard-anchor';

import { SequenceType } from '@blockworks-foundation/mangolana/lib/globalTypes';
import { sendSignAndConfirmTransactions } from '@blockworks-foundation/mangolana/lib/transactions';
import { AnchorProvider, Wallet } from 'switchboard-anchor';
import { Bank, TokenIndex } from '../src/accounts/bank';
import { Group } from '../src/accounts/group';
import { parseSwitchboardOracle } from '../src/accounts/oracle';
import { PerpMarketIndex } from '../src/accounts/perp';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID, MANGO_V4_MAIN_GROUP } from '../src/constants';
import { createComputeBudgetIx, createComputeLimitIx } from '../src/utils/rpc';
import { manageFeeWebSocket } from './manageFeeWs';
import {
  getOraclesForMangoGroup,
  OraclesFromMangoGroupInterface,
} from './sb-on-demand-crank-utils';
import {
  cycleDurationHistogram,
  exposePromMetrics,
  fetchIxDurationHistogram,
  fetchIxFailureCounter,
  fetchIxSuccessCounter,
  refreshFailureCounter,
  refreshSuccessCounter,
  relativeSlotsSinceLastUpdateHistogram,
  relativeVarianceSinceLastUpdateHistogram,
  sendTxCounter,
  sendTxErrorCounter,
  updateBlockhashSlotDurationHistogram,
  updateOracleAiDurationHistogram,
} from './sb-on-demand-metrics';

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
    tier: string;
    fallbackForOracle: PublicKey | undefined;
    tokenIndex: TokenIndex | undefined;
    perpMarketIndex: PerpMarketIndex | undefined;
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
/// without a restart within 3 minutes, result object will be dynamically
/// updated as well as the passed group
async function setupBackgroundRefresh(
  client: MangoClient,
  group: Group, /// modified periodically
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

  const GROUP_REFRESH_INTERVAL = 180_000; // refresh every 3 minutes
  const refreshGroup = async function (): Promise<void> {
    try {
      await group.reloadAll(client);
      refreshSuccessCounter.labels({ label: 'group.reloadAll' }).inc(1);
      result.oracles = await prepareCandidateOracles(
        client,
        group,
        sbOnDemandProgram,
        crossbarClient,
      );
      refreshSuccessCounter.labels({ label: 'prepareCandidateOracles' }).inc(1);
    } catch (err) {
      console.error('[refresh]', err);
      refreshFailureCounter.label({ err }).inc(1);
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
      // group is updated in place
      const { oracles } = refresh;

      const startedAt = Date.now();
      const [block, slot] = await Promise.all([
        // use finalized blockhash for faster timeouts on transactions
        client.connection.getLatestBlockhash('finalized'),
        // use processed slot for accurate staleness measurement
        client.connection.getSlot('processed'),
      ]);

      refreshSuccessCounter.labels({ label: 'getLatestBlockhash+getSlot' }).inc(1);
      const blockhashSlotUpdateAt = Date.now();
      updateBlockhashSlotDurationHistogram.observe(
        blockhashSlotUpdateAt - startedAt,
      );

      // refresh oracle accounts to know when each oracle was last updated
      // updates oracle in place
      await updateFilteredOraclesAis(client, sbOnDemandProgram, oracles);
      refreshSuccessCounter.labels({ label: 'updateFilteredOraclesAis' }).inc(1);
      const aisUpdatedAt = Date.now();
      updateOracleAiDurationHistogram.observe(
        aisUpdatedAt - blockhashSlotUpdateAt,
      );

      const staleOracles = await filterForStaleOracles(
        oracles,
        client,
        group,
        slot,
      );
      refreshSuccessCounter.labels({ label: 'filterForStaleOracles' }).inc(1);
      const staleFilteredAt = Date.now();

      const crossBarSims = await Promise.all(
        oracles.map((o) =>
          crossbarClient.simulateFeeds([
            new Buffer(o.parsedConfigs.feedHash).toString('hex'),
          ]),
        ),
      );
      refreshSuccessCounter.labels({ label: 'simulateFeeds' }).inc(1);
      const simulatedAt = Date.now();
      fetchIxDurationHistogram.observe(simulatedAt - staleFilteredAt);

      const varianceThresholdCrossedOracles =
        await filterForVarianceThresholdOracles(oracles, client, crossBarSims);
      refreshSuccessCounter.labels({ label: 'filterForVarianceThresholdOracles' }).inc(
        1,
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

      // can do 6 per tx
      const crankChunks: OracleMetaInterface[][] = chunk(oraclesToCrank, 6);

      // don't wait for switchboard API or delay retry
      crankChunks.map(async (oracleChunk) => {
        if (oracleChunk.length == 0) return;

        const numSignatures = 2;
        try {
          // TODO: don't ignore LUTS
          // TODO: investigate if more data can be prefetced (as was before)
          const [pullIx, _luts] = await PullFeed.fetchUpdateManyIx(
            sbOnDemandProgram as any,
            {
              feeds: oracleChunk.map((o) => new PublicKey(o.oracle.oraclePk)),
              numSignatures,
              crossbarClient,
              payer: user.publicKey,
            },
          );
          for (const oracle of oracleChunk) {
            fetchIxSuccessCounter.labels({ oracle: oracle.oracle.name }).inc(1);
          }
          const ixPreparedAt = Date.now();
          fetchIxDurationHistogram.observe(ixPreparedAt - simulatedAt);

          const lamportsPerCu_ = Math.min(
            Math.max(lamportsPerCu ?? 150_000, 150_000),
            500_000,
          );

          const cuLimit = 150_000 + oracleChunk.length * 30_000 * numSignatures;

          // no need to await, fire and forget
          sendSignAndConfirmTransactions({
            connection,
            wallet: new Wallet(user),
            backupConnections: [
              ...(LITE_RPC_URL
                ? [new Connection(LITE_RPC_URL!, 'recent')]
                : []),
              ...(CLUSTER_URL_2
                ? [new Connection(CLUSTER_URL_2!, 'recent')]
                : []),
            ],
            // fail rather quickly and retry submission from scratch
            // timeout using finalized to stay below switchboard oracle staleness limit
            timeoutStrategy: { block, startBlockCheckAfterSecs: 20 },
            transactionInstructions: [
              {
                instructionsSet: [
                  {
                    signers: [],
                    transactionInstruction:
                      createComputeBudgetIx(lamportsPerCu_),
                  },
                  {
                    signers: [],
                    transactionInstruction: createComputeLimitIx(cuLimit),
                  },
                  {
                    signers: [],
                    transactionInstruction: pullIx,
                  },
                ],
                sequenceType: SequenceType.Parallel,
              },
            ],
            config: {
              maxTxesInBatch: 10,
              autoRetry: false,
              logFlowInfo: false,
              // TODO disable alts for now
              // useVersionedTransactions: true,
            },
            callbacks: {
              afterEveryTxSend: function (data) {
                sendTxCounter.inc(1);
                const sentAt = Date.now();
                const total = (sentAt - startedAt) / 1000;
                cycleDurationHistogram.observe(sentAt - startedAt);
                const blockhashSlotUpdate =
                  (blockhashSlotUpdateAt - startedAt) / 1000;
                const aiUpdate = (aisUpdatedAt - blockhashSlotUpdateAt) / 1000;
                const staleFilter = (staleFilteredAt - aisUpdatedAt) / 1000;
                const simulate = (simulatedAt - staleFilteredAt) / 1000;
                const varianceFilter =
                  (varianceFilteredAt - simulatedAt) / 1000;
                const ixPrepare = (ixPreparedAt - varianceFilteredAt) / 1000;
                const timing = {
                  blockhashSlotUpdate,
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
              onError: function (err, notProcessedTransactions) {
                sendTxErrorCounter.labels(err).inc(1);
                console.error(
                  `[tx send] ${notProcessedTransactions.length} error(s) after ${(Date.now() - ixPreparedAt) / 1000}s ${JSON.stringify(err)}`,
                );
              },
            },
          }).catch((reason) => {
            sendTxErrorCounter
              .labels({ err: `prom rejected: ${JSON.stringify(reason)}` })
              .inc(1);
            console.error(
              `[tx send] promise rejected after ${(Date.now() - ixPreparedAt) / 1000}s ${JSON.stringify(reason)}`,
            );
          });
        } catch (err) {
          console.error(
            `[ix fetch] error after ${(Date.now() - varianceFilteredAt) / 1000}s ${JSON.stringify(err)}`,
          );
          fetchIxFailureCounter.labels({ err }).inc(1);
        }
      });

      await new Promise((r) => setTimeout(r, SLEEP_MS));
    } catch (err) {
      console.error('[main]', err);
      refreshFailureCounter.labels({ err }).inc(1);
    }
  }
})();
exposePromMetrics(Number(process.env.PORT!), process.env.BIND);

/**
 * prepares the instruction to update an individual oracle using the cached data on oracle
 */
async function _preparePullIx(
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

const VARIANCE_THRESHOLD_PCT_BY_TIER = {
  S: 0.5,
  AAA: 1,
  AA: 1,
  A: 1,
  'A-': 1,
  BBB: 2,
  BB: 2,
  B: 2,
  C: 4,
  D: Number.MAX_VALUE,
};

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
    const thresholdPct = VARIANCE_THRESHOLD_PCT_BY_TIER[item.oracle.tier];
    relativeVarianceSinceLastUpdateHistogram
      .labels({ oracle: item.oracle.name })
      .observe(changePct / thresholdPct);
    if (changePct > thresholdPct) {
      console.log(
        `[filter variance] ${item.oracle.name}, candidate: ${thresholdPct} < ${changePct}, ${simPrice}, ${res.price}`,
      );
      varianceThresholdCrossedOracles.push(item);
    } else {
      console.log(
        `[filter variance] ${item.oracle.name}, non-candidate: ${thresholdPct} > ${changePct}, ${simPrice}, ${res.price},`,
      );
    }
  }
  return varianceThresholdCrossedOracles;
}

async function filterForStaleOracles(
  filteredOracles: OracleMetaInterface[],
  client: MangoClient,
  group: Group,
  lastProcessedSlot: number,
): Promise<OracleMetaInterface[]> {
  const staleOracles = new Array<OracleMetaInterface>();
  for (const item of filteredOracles) {
    // we know that all these oracles are SBOD
    const res = await parseSwitchboardOracle(
      item.oracle.oraclePk,
      item.ai!,
      client.connection,
    );

    const slotsSinceLastUpdate = lastProcessedSlot - res.lastUpdatedSlot;
    // one iteration takes 10s, retry is every 20s
    // this allows for at least 2 retries until the oracle becomes stale
    const safetySeconds = 20 * 3 + 10;
    const safetySlots = safetySeconds * 2.5;
    const slotsUntilUpdate = item.decodedPullFeed.maxStaleness - safetySlots;
    relativeSlotsSinceLastUpdateHistogram
      .labels({ oracle: item.oracle.name })
      .observe(slotsSinceLastUpdate / slotsUntilUpdate);
    if (slotsSinceLastUpdate > slotsUntilUpdate) {
      console.log(
        `[filter stale] ${item.oracle.name}, candidate, ${slotsSinceLastUpdate} > ${slotsUntilUpdate}, ${lastProcessedSlot}`,
      );

      // check if oracle is fallback and primary is not stale
      if (item.oracle.fallbackForOracle) {
        const mainOraclePk = item.oracle.fallbackForOracle.toString();
        const [bank] = group.banksMapByOracle.get(mainOraclePk) as Bank[];
        // we need a working staleness check for every oracle type not only SBOD in this case
        // this info is up to date bc. of setupBackgroundRefresh
        if (!bank.isOracleStaleOrUnconfident(lastProcessedSlot + safetySlots)) {
          console.log(
            `[filter stale] fallback ${item.oracle.name}, non-candidate, bc. main oracle is up to date ${mainOraclePk}`,
          );
          // skip to save on gas, primary is good enough
          continue;
        }
      }

      // this oracle is stale and there's no fresh primary either
      staleOracles.push(item);
    } else {
      console.log(
        `[filter stale] ${item.oracle.name}, non-candidate, ${slotsSinceLastUpdate} < ${slotsUntilUpdate}, ${lastProcessedSlot}`
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

  // combine account info and remove non sbod owned oracles
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
        tier: 'S',
        fallbackForOracle: undefined,
        tokenIndex: undefined,
        perpMarketIndex: undefined,
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
      tier: 'S',
      fallbackForOracle: undefined,
      tokenIndex: undefined,
      perpMarketIndex: undefined,
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
}
