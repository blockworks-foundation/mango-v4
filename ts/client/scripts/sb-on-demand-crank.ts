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
  Oracle,
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

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const LITE_RPC_URL = process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP = process.env.GROUP_OVERRIDE || MANGO_V4_MAIN_GROUP.toBase58();
const SLEEP_MS = Number(process.env.SLEEP_MS) || 50_000; // 100s

console.log(`Starting with ${SLEEP_MS}`);
console.log(`${CLUSTER_URL}`);

// TODO use mangolana to send txs

interface OracleInterface {
  oracle: {
    oraclePk: PublicKey;
    name: string;
  };
  decodedPullFeed: any;
  ai: AccountInfo<Buffer> | null;
}

(async function main(): Promise<never> {
  const { group, client, connection, user, userProvider } = await setupMango();

  const { sbOnDemandProgram, crossbarClient, queue } = await setupSwitchboard(
    client,
  );

  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      // periodically check if we have new candidates on the group
      const filteredOracles = await prepareCandidateOracles(group, client);

      for (let i = 0; i < 10; i++) {
        const slot = await client.connection.getSlot('finalized');

        await updateFilteredOraclesAis(
          client.connection,
          sbOnDemandProgram,
          filteredOracles,
        );

        const staleOracles = await filterForStaleOracles(
          filteredOracles,
          client,
          slot,
        );

        const crossBarSims = await Promise.all(
          filteredOracles.map(
            async (fo) =>
              await crossbarClient.simulateFeeds([
                new Buffer(fo.decodedPullFeed.feedHash).toString('hex'),
              ]),
          ),
        );
        const varianceThresholdCrossedOracles =
          await filterForVarianceThresholdOracles(
            filteredOracles,
            client,
            crossBarSims,
          );
        const oraclesToCrank: OracleInterface[] = uniqWith(
          [...staleOracles, ...varianceThresholdCrossedOracles],
          function (a, b) {
            return a.oracle.oraclePk.equals(b.oracle.oraclePk);
          },
        );

        console.log(
          `- round candidates | Stale: ${staleOracles
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

        const pullIxs: TransactionInstruction[] = [];
        const lutOwners: (PublicKey | Oracle)[] = [];
        for (const oracle of oraclesToCrank) {
          await preparePullIx(
            sbOnDemandProgram,
            oracle,
            queue,
            lutOwners,
            pullIxs,
          );
        }

        const ixsChunks = chunk(shuffle(pullIxs), 2, false);
        try {
          // use mangolana
          await sendSignAndConfirmTransactions({
            connection,
            wallet: new Wallet(user),
            backupConnections: [new Connection(LITE_RPC_URL!, 'recent')],
            transactionInstructions: ixsChunks.map((txChunk) => ({
              instructionsSet: [
                {
                  signers: [],
                  transactionInstruction: createComputeBudgetIx(80000),
                },
                ...txChunk.map((tx) => ({
                  signers: [],
                  transactionInstruction: tx,
                })),
              ],
              sequenceType: SequenceType.Sequential,
            })),
            config: {
              maxRetries: 5,
              autoRetry: true,
              maxTxesInBatch: 20,
              logFlowInfo: false,
            },
            callbacks: {
              afterEveryTxSend: function (data) {
                console.log(` - https://solscan.io/tx/${data['txid']}`);
              },
            },
          });
        } catch (error) {
          console.log(`Error in sending tx, ${JSON.stringify(error)}`);
        }

        await new Promise((r) => setTimeout(r, SLEEP_MS));
      }
    } catch (error) {
      console.log(error);
    }
  }
})();

async function preparePullIx(
  sbOnDemandProgram,
  oracle: OracleInterface,
  queue: PublicKey,
  lutOwners: (PublicKey | Oracle)[],
  pullIxs: TransactionInstruction[],
): Promise<void> {
  const pullFeed = new PullFeed(
    sbOnDemandProgram as any,
    new PublicKey(oracle.oracle.oraclePk),
  );

  const conf = {
    numSignatures: 2,
    feed: oracle.oracle.oraclePk,
  };
  // TODO use fetchUpdateMany
  const [pullIx, responses, success] = await pullFeed.fetchUpdateIx(conf);

  if (pullIx === undefined) {
    return;
  }

  // TODO
  // > Mitch | Switchboard:
  // there can be more oracles that join a queue over time
  // all oracles and feeds carry their own LUT as im sure you noticed
  // > Mitch | Switchboard:
  // the feed ones are easy to predict though
  // > Mitch | Switchboard:
  // but you dont know which oracles the gateway will select for you so best you can do is pack all oracle accounts into 1lut

  const lutOwners_ = [...responses.map((x) => x.oracle), pullFeed.pubkey];
  lutOwners.push(...lutOwners_);

  pullIxs.push(pullIx!);
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
    const changeBps = changePct * 100;
    if (changePct > item.decodedPullFeed.maxVariance / 1000000000) {
      console.log(
        `- ${item.oracle.name}, candidate, ${
          item.decodedPullFeed.maxVariance / 1000000000
        }, ${simPrice}, ${res.price}, ${changePct}`,
      );
      varianceThresholdCrossedOracles.push(item);
    } else {
      console.log(
        `- ${item.oracle.name}, non-candidate, ${
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
      slot - res.lastUpdatedSlot > (item.decodedPullFeed.maxStaleness * 8) / 10
    ) {
      console.log(
        `- ${item.oracle.name}, candidate, ${item.decodedPullFeed.maxStaleness}, ${slot}, ${res.lastUpdatedSlot}, ${diff}`,
      );
      staleOracles.push(item);
    } else {
      console.log(
        `- ${item.oracle.name}, non-candidate, ${item.decodedPullFeed.maxStaleness}, ${slot}, ${res.lastUpdatedSlot}, ${diff}`,
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
    ['DIGITSOL', '2A7aqNLy26ZBSMWP2Ekxv926hj16tCA47W1sHWVqaLii'],
    ['JLP', '65J9bVEMhNbtbsNgArNV1K4krzcsomjho4bgR51sZXoj'],
    ['INF', 'AZcoqpWhMJUaKEDUfKsfzCr3Y96gSQwv43KSQ6KpeyQ1'],
    ['GUAC', 'Ai2GsLRioGKwVgWX8dtbLF5rJJEZX17SteGEDqrpzBv3'],
    ['RAY', 'AJkAFiXdbMonys8rTXZBrRnuUiLcDFdkyoPuvrVKXhex'],
    ['JUP', '2F9M59yYc28WMrAymNWceaBEk8ZmDAjUAKULp8seAJF3'],
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
      idsSource: 'get-program-accounts',
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
