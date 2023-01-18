import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Cluster, Connection, Keypair } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
dotenv.config();

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const PAYER_KEYPAIR =
  process.env.PAYER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_NUM = Number(process.env.GROUP_NUM || 0);
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(PAYER_KEYPAIR!, 'utf-8'))),
  );

  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = MangoClient.connect(
    adminProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    { idsSource: 'get-program-accounts' },
  );

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  const mangoAccounts = await client.getAllMangoAccounts(group);

  Array.from(group.perpMarketsMapByMarketIndex.values())
    .filter((perpMarket) => perpMarket.name != 'SOMETHING-PERP')
    .map((perpMarket) => {
      console.log(`name ${perpMarket.name}`);
      let getUnsettledPnlUiAgg = 0;
      let getBasePositionUiAgg = 0;
      let longSettledFundingAgg = 0;
      let shortSettledFundingAgg = 0;
      mangoAccounts.map((mangoAccount) => {
        const pp = mangoAccount
          .perpActive()
          .find((pp) => pp.marketIndex === perpMarket.perpMarketIndex);
        if (pp) {
          getUnsettledPnlUiAgg += pp.getUnsettledPnlUi(group, perpMarket);
          getBasePositionUiAgg += pp.getBasePositionUi(perpMarket);
          longSettledFundingAgg += pp.longSettledFunding.toNumber();
          shortSettledFundingAgg += pp.shortSettledFunding.toNumber();
          console.log(` - ${mangoAccount.publicKey.toBase58().padStart(45)}`);
          console.log(
            `    - unsettled pnl ${pp
              .getUnsettledPnlUi(group, perpMarket)
              .toFixed(4)
              .padStart(10)}`,
          );
          console.log(
            `    - base position ${pp
              .getBasePositionUi(perpMarket)
              .toFixed(4)
              .padStart(10)}`,
          );
          // console.log(
          //   `    - avgEntryPricePerBaseLot ${pp.avgEntryPricePerBaseLot}`,
          // );
          // console.log(
          //   `    - realizedTradePnl ${toUiDecimalsForQuote(
          //     pp.realizedTradePnlNative,
          //   )}`,
          // );
          // console.log(
          //   `    - realizedOtherPnl ${toUiDecimalsForQuote(
          //     pp.realizedOtherPnlNative,
          //   )}`,
          // );
          // console.log(
          //   `    - settlePnlLimitRealizedTrade ${pp.settlePnlLimitRealizedTrade.toNumber()}`,
          // );
          // console.log(
          //   `    - realizedPnlForPosition ${toUiDecimalsForQuote(
          //     pp.realizedPnlForPositionNative,
          //   )}`,
          // );
          // console.log(
          //   `    - settlePnlLimitSettledInCurrentWindow ${toUiDecimalsForQuote(
          //     pp.settlePnlLimitSettledInCurrentWindowNative,
          //   )}`,
          // );
        }
      });
      // console.log(
      //   `- feesAccrued ${toUiDecimalsForQuote(perpMarket.feesAccrued)}`,
      // );
      // console.log(
      //   `- feesSettled ${toUiDecimalsForQuote(perpMarket.feesSettled)}`,
      // );
      // console.log(
      //   `- longSettledFundingAgg ${longSettledFundingAgg
      //     .toFixed(4)
      //     .padStart(10)}`,
      // );
      // console.log(
      //   `- shortSettledFunding ${shortSettledFundingAgg
      //     .toFixed(4)
      //     .padStart(10)}`,
      // );
      console.log(
        `- unsettled pnl aggr ${getUnsettledPnlUiAgg.toFixed(4).padStart(10)}`,
      );
      console.log(
        `- base position aggr ${getBasePositionUiAgg.toFixed(4).padStart(10)}`,
      );
      console.log(
        `- base position aggr * price ${(
          getBasePositionUiAgg * perpMarket.uiPrice
        )
          .toFixed(4)
          .padStart(10)}`,
      );
      console.log(
        `- unsettled pnl aggr - base position aggr * price ${(
          getUnsettledPnlUiAgg -
          getBasePositionUiAgg * perpMarket.uiPrice
        )
          .toFixed(4)
          .padStart(10)}`,
      );
      console.log();
    });

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
