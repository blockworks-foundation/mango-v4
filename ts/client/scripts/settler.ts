import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = new PublicKey(
  process.env.MANGO_ACCOUNT_PK || PublicKey.default.toBase58(),
);
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const kp = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(USER_KEYPAIR!, 'utf-8'))),
  );
  const wallet = new Wallet(kp);
  const provider = new AnchorProvider(connection, wallet, options);
  const client = MangoClient.connect(provider, CLUSTER, MANGO_V4_ID[CLUSTER], {
    idsSource: 'api',
  });

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  let mangoAccounts = await client.getAllMangoAccounts(group, true);

  const settler = await client.getMangoAccountForOwner(
    group,
    wallet.publicKey,
    0,
    true,
  );

  if (MANGO_ACCOUNT_PK) {
    const mangoAccount = await client.getMangoAccount(
      new PublicKey(MANGO_ACCOUNT_PK),
    );

    for (const pp of mangoAccount.perpActive()) {
      // settle only a specific market
      // if (pp.marketIndex != 2) continue;

      const pm = group.getPerpMarketByMarketIndex(pp.marketIndex);
      const upnlUi = pp.getUnsettledPnlUi(pm);
      if (upnlUi > 0) {
        const c = await pm.getSettlePnlCandidates(
          client,
          group,
          mangoAccounts,
          'negative',
        );
        const sig = await client.perpSettlePnl(
          group,
          mangoAccount,
          c[0].account,
          settler!,
          pm.perpMarketIndex,
        );
        console.log(sig);
      } else {
        const c = await pm.getSettlePnlCandidates(
          client,
          group,
          mangoAccounts,
          'positive',
        );
        const sig = await client.perpSettlePnl(
          group,
          c[0].account,
          mangoAccount,
          settler!,
          pm.perpMarketIndex,
        );
        console.log(sig);
      }
    }

    process.exit();
  }

  // TODO settle perp pnl for all positions which have been closed but not deactivated
  // console.log(mangoAccounts.length);
  // mangoAccounts = mangoAccounts.filter(
  //   (a) =>
  //     a.perpActive().length > 0 &&
  //     a
  //       .perpActive()
  //       .some(
  //         (pp) =>
  //           pp.getBasePositionUi(
  //             group.getPerpMarketByMarketIndex(pp.marketIndex),
  //           ) == 0 &&
  //           Math.abs(
  //             pp.getUnsettledFundingUi(
  //               group.getPerpMarketByMarketIndex(pp.marketIndex),
  //             ),
  //           ) > 0,
  //       ) === true,
  // );

  process.exit();
}

main();
