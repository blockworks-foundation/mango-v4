import {
  AccountInfo,
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  CrossbarClient,
  PullFeed,
  SB_ON_DEMAND_PID,
} from '@switchboard-xyz/on-demand';
import fs from 'fs';
import chunk from 'lodash/chunk';
import shuffle from 'lodash/shuffle';
import uniqWith from 'lodash/uniqWith';
import { Program as Anchor30Program, Idl } from 'switchboard-anchor';

import { SequenceType } from '@blockworks-foundation/mangolana/lib/globalTypes';
import { sendSignAndConfirmTransactions } from '@blockworks-foundation/mangolana/lib/transactions';
import { AnchorProvider, Wallet } from 'switchboard-anchor';
import { Group } from '../src/accounts/group';
import { parseSwitchboardOracle } from '../src/accounts/oracle';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID, MANGO_V4_MAIN_GROUP } from '../src/constants';
import { ZERO_I80F48 } from '../src/numbers/I80F48';
import { createComputeBudgetIx } from '../src/utils/rpc';
import { manageFeeWebSocket } from './manageFeeWs';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const CLUSTER_URL_2 = process.env.MB_CLUSTER_URL_2;
const LITE_RPC_URL = process.env.LITE_RPC_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP = process.env.GROUP_OVERRIDE || MANGO_V4_MAIN_GROUP.toBase58();
const SLEEP_MS = Number(process.env.SLEEP_MS) || 50_000;

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

interface OracleInterface {
  oracle: {
    oraclePk: PublicKey;
    name: string;
  };
  decodedPullFeed: any;
  ai: AccountInfo<Buffer> | null;
}

(async function main(): Promise<never> {
  const { group, client, connection, user } = await setupMango();

  const { sbOnDemandProgram, crossbarClient } =
    await setupSwitchboard(client);


  while (true) {
    try {
      // periodically check if we have new candidates on the group
      const filteredOracles = await prepareCandidateOracles(group, client);

      for (let i = 0; i < 10; i++) {
        const startedAt = Date.now();
        const slot = await client.connection.getSlot('finalized');

        await updateFilteredOraclesAis(
          client.connection,
          sbOnDemandProgram,
          filteredOracles,
        );

        const aisUpdatedAt = Date.now();

        const staleOracles = await filterForStaleOracles(
          filteredOracles,
          client,
          slot,
        );

        const staleFilteredAt = Date.now();

        const crossBarSims = await Promise.all(
          filteredOracles.map(
            async (fo) =>
              await crossbarClient.simulateFeeds([
                new Buffer(fo.decodedPullFeed.feedHash).toString('hex'),
              ]),
          ),
        );

        const simulatedAt = Date.now();

        const varianceThresholdCrossedOracles =
          await filterForVarianceThresholdOracles(
            filteredOracles,
            client,
            crossBarSims,
          );

        const varianceFilteredAt = Date.now();

        const oraclesToCrank: OracleInterface[] = uniqWith(
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

        const pullIxs = (
          await Promise.all(
            oraclesToCrank.map(async (oracle) => {
              const pullIx = await preparePullIx(sbOnDemandProgram, oracle);
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
        try {
          // dont await, fire and forget
          // TODO use our own ALTs
          sendSignAndConfirmTransactions({
            connection,
            wallet: new Wallet(user),
            backupConnections: [
              ...(CLUSTER_URL_2
                ? [new Connection(LITE_RPC_URL!, 'recent')]
                : []),
              ...(CLUSTER_URL_2
                ? [new Connection(CLUSTER_URL_2!, 'recent')]
                : []),
            ],
            transactionInstructions: ixsChunks.map((txChunk) => ({
              instructionsSet: [
                {
                  signers: [],
                  transactionInstruction: createComputeBudgetIx(lamportsPerCu_),
                },
                ...txChunk.map((tx) => ({
                  signers: [],
                  transactionInstruction: tx,
                })),
              ],
              sequenceType: SequenceType.Parallel,
            })),
            config: {
              maxTxesInBatch: 1,
              maxRetries: 5,
              autoRetry: true,
              logFlowInfo: false,
            },
            callbacks: {
              afterEveryTxSend: function (data) {
                const sentAt = Date.now();
                const total = (sentAt - startedAt) / 1000;
                const aiUpdate = (aisUpdatedAt - startedAt) / 1000;
                const staleFilter = (staleFilteredAt - aisUpdatedAt) / 1000;
                const simulate = (simulatedAt - staleFilteredAt) / 1000;
                const varianceFilter =
                  (varianceFilteredAt - simulatedAt) / 1000;
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
              onError: function (e, notProcessedTransactions, _originalProps) {
                console.error(
                  '[tx send] num transactions:',
                  notProcessedTransactions.length,
                  e,
                );
              },
            },
          });
        } catch (error) {
          console.error(
            `[tx send] ${JSON.stringify(error.message)}, https://solscan.io/tx/${error['txid']}, in ${(Date.now() - startedAt) / 1000}s, lamportsPerCu_ ${lamportsPerCu_}, lamportsPerCu ${lamportsPerCu}`,
          );
        }

        await new Promise((r) => setTimeout(r, SLEEP_MS));
      }
    } catch (error) {
      console.error('[main]', error);
    }
  }
})();

async function preparePullIx(
  sbOnDemandProgram,
  oracle: OracleInterface,
): Promise<TransactionInstruction | undefined> {
  const pullFeed = new PullFeed(
    sbOnDemandProgram as any,
    new PublicKey(oracle.oracle.oraclePk),
  );

  const conf = {
    numSignatures: oracle.decodedPullFeed.minResponses,
    feed: oracle.oracle.oraclePk,
  };
  // TODO use fetchUpdateMany
  const [pullIx, _responses, _success] = await pullFeed.fetchUpdateIx(conf);

  return pullIx;
}

async function filterForVarianceThresholdOracles(
  filteredOracles: OracleInterface[],
  client: MangoClient,
  crossBarSims,
): Promise<OracleInterface[]> {
  const varianceThresholdCrossedOracles = new Array<OracleInterface>();
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
  filteredOracles: OracleInterface[],
  client: MangoClient,
  slot: number,
): Promise<OracleInterface[]> {
  const staleOracles = new Array<OracleInterface>();
  for (const item of filteredOracles) {
    const res = await parseSwitchboardOracle(
      item.oracle.oraclePk,
      item.ai!,
      client.connection,
    );

    const diff = slot - res.lastUpdatedSlot;
    if (
      slot > res.lastUpdatedSlot &&
      slot - res.lastUpdatedSlot > (item.decodedPullFeed.maxStaleness * 1) / 2
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

async function prepareCandidateOracles(
  group: Group,
  client: MangoClient,
): Promise<OracleInterface[]> {
  const oracles = getOraclesForMangoGroup(group);
  oracles.push(...extendOraclesManually(CLUSTER));

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

  const filteredOracles = oracles
    .map((o, i) => {
      return { oracle: o, ai: ais[i], decodedPullFeed: undefined };
    })
    .filter((item) => item.ai?.owner.equals(SB_ON_DEMAND_PID));

  return filteredOracles;
}

function extendOraclesManually(cluster: Cluster): {
  oraclePk: PublicKey;
  name: string;
}[] {
  if (cluster == 'devnet') {
    return [
      {
        oraclePk: new PublicKey('EtbG8PSDCyCSmDH8RE4Nf2qTV9d6P6zShzHY2XWvjFJf'),
        name: 'BTC/USD',
      },
    ];
  }
  return [
    ['JSOL/USD', 'Dnn9fKeB3rA2bor6Fys7FBPqXneAK8brxNfsBfZ32939'],
    ['compassSOL/USD', 'GzBpasKMSTLkytXpyo6NesDGpe2mLjPSovECWsebQpu5'],
    ['dualSOL/USD', 'D6UqFgtVC1yADBxw2EZFmUCTNuoqFoUXD3NW4NqRn8v3'],
    ['hubSOL/USD', '7LRVXc8zdPpzXNdknU2kRTYt7BizYs7BaM6Ft2zv8E4h'],
    ['hubSOL/USD', '137fd2LnDEPVAALhPFjRyvh2MD9DxSHPFaod7a5tmMox'],
    ['digitSOL/USD', '7skmP8qLf8KKJ61cpPiw91GXYfoGvGWekzSDQ78T3z1f'],
    ['mangoSOL/USD', '7pD4Y1hCsU4M6rfoJvL8fAmmrB2LwrJYxvWz4S6Cc24T'],
  ].map((item) => {
    return {
      oraclePk: new PublicKey(item[1]),
      name: item[0],
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
  const options = AnchorProvider.defaultOptions();
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

function getOraclesForMangoGroup(
  group: Group,
): { oraclePk: PublicKey; name: string }[] {
  // oracles for tokens
  const oracles1 = Array.from(group.banksMapByName.values())
    .filter(
      (b) =>
        !(
          b[0].nativeDeposits().eq(ZERO_I80F48()) &&
          b[0].nativeBorrows().eq(ZERO_I80F48()) &&
          b[0].reduceOnly == 1
        ),
    )
    .map((b) => {
      return {
        oraclePk: b[0].oracle,

        name: b[0].name,
      };
    });

  // oracles for perp markets
  const oracles2 = Array.from(group.perpMarketsMapByName.values()).map((pM) => {
    return {
      oraclePk: pM.oracle,

      name: pM.name,
    };
  });

  // fallback oracles for tokens
  const oracles3 = Array.from(group.banksMapByName.values())
    .filter(
      (b) =>
        !(
          b[0].nativeDeposits().eq(ZERO_I80F48()) &&
          b[0].nativeBorrows().eq(ZERO_I80F48()) &&
          b[0].reduceOnly == 1
        ),
    )
    .map((b) => {
      return {
        oraclePk: b[0].oracle,

        name: b[0].name,
      };
    })
    .filter((item) => !item.oraclePk.equals(PublicKey.default));
  const oracles = oracles1.concat(oracles2).concat(oracles3);
  return oracles;
}

async function setupSwitchboard(client: MangoClient): Promise<{
  sbOnDemandProgram: Anchor30Program<Idl>;
  crossbarClient: CrossbarClient;
  queue: PublicKey;
}> {
  const idl = await Anchor30Program.fetchIdl(
    SB_ON_DEMAND_PID,
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

async function updateFilteredOraclesAis(
  connection: Connection,
  sbOnDemandProgram: Anchor30Program<Idl>,
  filteredOracles: OracleInterface[],
): Promise<void> {
  const ais = (
    await Promise.all(
      chunk(
        filteredOracles.map((item) => item.oracle.oraclePk),
        50,
        false,
      ).map(async (chunk) => await connection.getMultipleAccountsInfo(chunk)),
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
