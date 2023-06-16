import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { toUiDecimalsForQuote } from '../src/utils';
dotenv.config();

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const GROUP_PK =
  process.env.GROUP_PK || '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  const wallet = new Wallet(new Keypair());
  const provider = new AnchorProvider(connection, wallet, options);
  const client = MangoClient.connect(provider, CLUSTER, MANGO_V4_ID[CLUSTER], {
    idsSource: 'get-program-accounts',
  });

  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const mangoAccounts = await client.getAllMangoAccounts(group, true);

  Array.from(group.perpMarketsMapByMarketIndex.values())
    .filter((perpMarket) => perpMarket.name != 'SOMETHING-PERP')
    .map((perpMarket) => {
      console.log(`name ${perpMarket.name}`);
      let getUnsettledPnlUiAgg = 0;
      let getBasePositionUiAgg = 0;
      let getQuotePositionUiAgg = 0;
      let longSettledFundingAgg = 0;
      let shortSettledFundingAgg = 0;
      mangoAccounts.map((mangoAccount) => {
        const pp = mangoAccount
          .perpActive()
          .find((pp) => pp.marketIndex === perpMarket.perpMarketIndex);
        if (pp) {
          getUnsettledPnlUiAgg +=
            pp.getUnsettledPnlUi(perpMarket) -
            pp.getUnsettledFundingUi(perpMarket);
          getBasePositionUiAgg += pp.getBasePositionUi(perpMarket);
          getQuotePositionUiAgg += pp.getQuotePositionUi(perpMarket);
          longSettledFundingAgg += pp.longSettledFunding.toNumber();
          shortSettledFundingAgg += pp.shortSettledFunding.toNumber();
        }
      });

      // console.log(
      //   `- longSettledFundingAgg - shortSettledFunding ${(
      //     longSettledFundingAgg - shortSettledFundingAgg
      //   )
      //     .toFixed(4)
      //     .padStart(10)}`,
      // );
      // console.log(
      //   `- unsettled pnl aggr ${getUnsettledPnlUiAgg.toFixed(4).padStart(10)}`,
      // );
      // console.log(
      //   `- base position aggr ${getBasePositionUiAgg.toFixed(4).padStart(10)}`,
      // );
      // console.log(
      //   `- quote position aggr ${getQuotePositionUiAgg
      //     .toFixed(4)
      //     .padStart(10)}`,
      // );
      // console.log(
      //   `- base position aggr * price ${(
      //     getBasePositionUiAgg * perpMarket.uiPrice
      //   )
      //     .toFixed(4)
      //     .padStart(10)}`,
      // );
      // console.log(
      //   `- perp.feesAccrued ${toUiDecimalsForQuote(perpMarket.feesAccrued)}`,
      // );
      // console.log(
      //   `- unsettled pnl aggr - base position aggr * price ${(
      //     getUnsettledPnlUiAgg -
      //     getBasePositionUiAgg * perpMarket.uiPrice
      //   )
      //     .toFixed(4)
      //     .padStart(10)}`,
      // );
      console.log(
        `- perp.feesAccrued  + unsettled pnl aggr ${
          toUiDecimalsForQuote(perpMarket.feesAccrued) + getUnsettledPnlUiAgg
        }`,
      );
    });

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
