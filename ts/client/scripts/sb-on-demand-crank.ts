import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
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
import uniq from 'lodash/uniq';
import { Program as Anchor30Program } from 'switchboard-anchor';

import { OracleConfig } from '../src/accounts/bank';
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

// eslint-disable-next-line @typescript-eslint/explicit-function-return-type
(async function main() {
  ///
  /// Wallet+Client setup
  ///
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

  ///
  /// Prepare all oracles we want to crank
  ///

  // TODO reload group once in a while

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

  /// Manually exclude some
  // TODO

  /// Manually include some
  oracles.push({
    oraclePk: new PublicKey('EtbG8PSDCyCSmDH8RE4Nf2qTV9d6P6zShzHY2XWvjFJf'),
    oracleConfig: {
      confFilter: I80F48.fromString('0.1'),
      maxStalenessSlots: new BN(5),
    },
    name: 'BTC/USD',
  });

  /// Maybe support more than one mango group
  // TODO

  /// Maybe support additional via csv env param
  // TODO

  ///
  /// Filter for sb on demand oracles
  ///
  // TODO ensure ai is not null
  const ais = await client.program.provider.connection.getMultipleAccountsInfo(
    oracles.map((item) => item.oraclePk),
  );
  const filteredOracles = oracles
    .map((o, i) => {
      return { oracle: o, ai: ais[i] };
    })
    .filter((item) => item.ai?.owner.equals(SB_ON_DEMAND_PID));

  ///
  /// sb
  ///
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
    /* verbose= */ true,
  );

  ///
  /// Loop indefinitely
  ///
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const slot = await client.connection.getSlot();

    // filter candidates for this iteration

    // 1. stale
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

      if (slot > res.lastUpdatedSlot) {
        if (
          slot - res.lastUpdatedSlot >
          item.oracle.oracleConfig.maxStalenessSlots.toNumber()
        ) {
          staleOracles.push(item);
        }
      }
    }

    // 2. variance
    const varianceThresholdCrossedOracles = new Array<{
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

      const decodedPullFeed = sbOnDemandProgram.coder.accounts.decode(
        'pullFeedAccountData',
        item.ai!.data,
      );

      const crossBarSim = await crossbarClient.simulateFeeds([
        new Buffer(decodedPullFeed.feedHash).toString('hex'),
      ]);

      const simPrice =
        crossBarSim[0].results.reduce((a, b) => a + b, 0) /
        crossBarSim[0].results.length;

      if ((res.price - simPrice) / res.price > 0.01) {
        varianceThresholdCrossedOracles.push(item);
      }
    }

    // 3. stale or variance
    // TODO verify this works
    const oraclesToCrank = uniq(
      [...staleOracles, ...varianceThresholdCrossedOracles],
      function (item) {
        return item.oracle.oraclePk.toString();
      },
    );

    /// Build pull ixs
    const pullIxs: TransactionInstruction[] = [];
    const lutOwners: (PublicKey | Oracle)[] = [];
    for (const oracle of oraclesToCrank) {
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
        maxVariance: decodedPullFeed.maxVariance.toNumber(),
        minResponses: decodedPullFeed.minResponses,
        numSignatures: 3, // TODO hardcoded
        minSampleSize: decodedPullFeed.minSampleSize,
        maxStaleness: decodedPullFeed.maxStaleness,
      };
      const [pullIx, responses, success] = await pullFeed.fetchUpdateIx(conf);

      const lutOwners_ = [...responses.map((x) => x.oracle), pullFeed.pubkey];
      lutOwners.push(...lutOwners_);
      pullIxs.push(pullIx!);
    }

    for (const c of chunk(pullIxs, 5)) {
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

      const sim = await client.connection.simulateTransaction(tx, txOpts);
      const sig = await client.connection.sendTransaction(tx, txOpts);
      console.log(`updated in ${sig}`); // TODO add token names
    }

    await new Promise((r) => setTimeout(r, 5000));
  }
})();
