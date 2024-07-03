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
  asV0Tx,
  CrossbarClient,
  loadLookupTables,
  Oracle,
  PullFeed,
  SB_ON_DEMAND_PID,
} from '@switchboard-xyz/on-demand';
import fs from 'fs';
import chunk from 'lodash/chunk';
import uniqWith from 'lodash/uniqWith';
import { Program as Anchor30Program, Idl } from 'switchboard-anchor';

import BN from 'bn.js';
import { AnchorProvider, Wallet } from 'switchboard-anchor';
import { OracleConfig } from '../src/accounts/bank';
import { Group } from '../src/accounts/group';
import { parseSwitchboardOracle } from '../src/accounts/oracle';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID, MANGO_V4_MAIN_GROUP } from '../src/constants';
import { I80F48, ZERO_I80F48 } from '../src/numbers/I80F48';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP = process.env.GROUP_OVERRIDE || MANGO_V4_MAIN_GROUP.toBase58();
const SLEEP_MS = Number(process.env.SLEEP_MS) || 5_000;

// TODO use mangolana to send txs

(async function main(): Promise<never> {
  const { group, client, connection, user } = await setupMango();

  const { sbOnDemandProgram, crossbarClient, queue } = await setupSwitchboard(
    client,
  );

  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      // periodically check if we have new candidates on the group
      const filteredOracles = await prepareCandidateOracles(group, client);

      for (let i = 0; i < 10; i++) {
        const slot = await client.connection.getSlot();

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

        const varianceThresholdCrossedOracles =
          await filterForVarianceThresholdOracles(
            filteredOracles,
            client,
            crossbarClient,
          );

        const oraclesToCrank = uniqWith(
          [...staleOracles, ...varianceThresholdCrossedOracles],
          function (item) {
            return item.oracle.oraclePk.toString();
          },
        );

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

        for (const c of chunk(pullIxs, 5, false)) {
          const tx = await asV0Tx({
            connection,
            ixs: [...c],
            signers: [user],
            computeUnitPrice: 200_000,
            computeUnitLimitMultiple: 1.3,
            lookupTables: await loadLookupTables(lutOwners),
          });

          const txOpts = {
            commitment: 'processed' as Commitment,
            skipPreflight: true,
            maxRetries: 0,
          };

          const sig = await client.connection.sendTransaction(tx, txOpts);
          console.log(`submitted in in https://solscan.io/tx/${sig}`);
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
  oracle: any,
  queue: PublicKey,
  lutOwners: (PublicKey | Oracle)[],
  pullIxs: TransactionInstruction[],
): Promise<void> {
  const pullFeed = new PullFeed(
    sbOnDemandProgram as any,
    new PublicKey(oracle.oracle.oraclePk),
  );

  const decodedPullFeed = sbOnDemandProgram.coder.accounts.decode(
    'pullFeedAccountData',
    oracle.ai.data,
  );

  const conf = {
    queue: queue,
    numSignatures: 3,
    feedHash: decodedPullFeed.feedHash,
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
  filteredOracles: {
    oracle: { oraclePk: PublicKey; oracleConfig: OracleConfig; name: string };
    decodedPullFeed: any;
    ai: AccountInfo<Buffer> | null;
  }[],
  client: MangoClient,
  crossbarClient: CrossbarClient,
): Promise<
  {
    oracle: {
      oraclePk: PublicKey;
      oracleConfig: OracleConfig;
    };
    ai: AccountInfo<Buffer> | null;
  }[]
> {
  const varianceThresholdCrossedOracles = new Array<{
    oracle: {
      oraclePk: PublicKey;
      oracleConfig: OracleConfig;
    };
    decodedPullFeed: any;
    ai: AccountInfo<Buffer> | null;
  }>();
  for (const item of filteredOracles) {
    const res = await parseSwitchboardOracle(
      item.oracle.oraclePk,
      item.ai!,
      client.connection,
    );

    const crossBarSim = await crossbarClient.simulateFeeds([
      new Buffer(item.decodedPullFeed.feedHash).toString('hex'),
    ]);

    const simPrice =
      crossBarSim[0].results.reduce((a, b) => a + b, 0) /
      crossBarSim[0].results.length;

    if (Math.abs(res.price - simPrice) / res.price > 0.01) {
      console.log(
        `- Variance threshold crossed oracle, candidate ${
          item.oracle.name
        } ${simPrice} ${res.price} ${(
          (Math.abs(res.price - simPrice) * 10000) /
          res.price
        ).toFixed()} bps`,
      );
      varianceThresholdCrossedOracles.push(item);
    } else {
      console.log(
        `- Variance threshold crossed oracle, non candidate ${
          item.oracle.name
        } ${simPrice} ${res.price} ${(
          (Math.abs(res.price - simPrice) * 10000) /
          res.price
        ).toFixed()} bps`,
      );
    }
  }
  return varianceThresholdCrossedOracles;
}

async function filterForStaleOracles(
  filteredOracles: {
    oracle: { oraclePk: PublicKey; oracleConfig: OracleConfig; name: string };
    decodedPullFeed: any;
    ai: AccountInfo<Buffer> | null;
  }[],
  client: MangoClient,
  slot: number,
): Promise<
  {
    oracle: {
      oraclePk: PublicKey;
      oracleConfig: OracleConfig;
    };
    ai: AccountInfo<Buffer> | null;
  }[]
> {
  const staleOracles = new Array<{
    oracle: {
      oraclePk: PublicKey;
      oracleConfig: OracleConfig;
    };
    ai: AccountInfo<Buffer> | null;
  }>();
  for (const item of filteredOracles) {
    const res = await parseSwitchboardOracle(
      item.oracle.oraclePk,
      item.ai!,
      client.connection,
    );

    if (
      slot > res.lastUpdatedSlot &&
      slot - res.lastUpdatedSlot > item.decodedPullFeed.maxStaleness
    ) {
      console.log(
        `- Stale oracle, candidate ${item.oracle.name} ${slot} ${
          item.decodedPullFeed.maxStaleness
        } ${res.lastUpdatedSlot} ${slot - res.lastUpdatedSlot}`,
      );
      staleOracles.push(item);
    } else {
      console.log(
        `- Stale oracle, non candidate ${item.oracle.name} ${slot} ${
          res.lastUpdatedSlot
        } ${res.lastUpdatedSlot} ${slot - res.lastUpdatedSlot}`,
      );
    }
  }
  return staleOracles;
}

async function prepareCandidateOracles(
  group: Group,
  client: MangoClient,
): Promise<
  {
    oracle: { oraclePk: PublicKey; oracleConfig: OracleConfig; name: string };
    decodedPullFeed: any;
    ai: AccountInfo<Buffer> | null;
  }[]
> {
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
      return { oracle: o, ai: ais[i] };
    })
    .filter((item) => item.ai?.owner.equals(SB_ON_DEMAND_PID));

  return filteredOracles;
}

function extendOraclesManually(cluster: Cluster) {
  if (cluster == 'devnet') {
    return [
      {
        oraclePk: new PublicKey('EtbG8PSDCyCSmDH8RE4Nf2qTV9d6P6zShzHY2XWvjFJf'),
        oracleConfig: {
          confFilter: I80F48.fromString('0.1'),
          maxStalenessSlots: new BN(5),
        },
        name: 'BTC/USD',
      },
    ];
  }
  return [
    {
      // https://ondemand.switchboard.xyz/solana/mainnet/user/8SSLjXBEVk9nesbhi9UMCA32uijbVBUqWoKPPQPTekzt/
      oraclePk: new PublicKey('31VbxqvoswUh6iWwv92mXVdLz7vt2QGV1DZ1tBa64b8w'),
      oracleConfig: {
        confFilter: I80F48.fromString('1000'),
        maxStalenessSlots: new BN(-1),
      },
      name: 'MNGO/USD',
    },
  ];
}

async function setupMango(): Promise<{
  group: Group;
  client: MangoClient;
  connection: Connection;
  user: Keypair;
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
  return { group, client, connection, user };
}

function getOraclesForMangoGroup(group: Group) {
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
        oracleConfig: b[0].oracleConfig,
        name: b[0].name,
      };
    });

  // oracles for perp markets
  const oracles2 = Array.from(group.perpMarketsMapByName.values()).map((pM) => {
    return {
      oraclePk: pM.oracle,
      oracleConfig: pM.oracleConfig,
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
        oracleConfig: b[0].oracleConfig,
        name: b[0].name,
      };
    })
    .filter((item) => !item.oraclePk.equals(PublicKey.default));
  const oracles = oracles1.concat(oracles2).concat(oracles3);
  return oracles;
}

async function setupSwitchboard(client: MangoClient) {
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
  filteredOracles: {
    decodedPullFeed: any;
    oracle: { oraclePk: PublicKey; oracleConfig: OracleConfig; name: string };
    ai: AccountInfo<Buffer> | null;
  }[],
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
