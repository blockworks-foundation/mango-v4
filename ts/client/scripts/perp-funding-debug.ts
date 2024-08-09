import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../src/client';

async function main(): Promise<void> {
  const client = await MangoClient.connectDefault(process.env.MB_CLUSTER_URL!);

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const group = await client.getGroup(
      new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
    );

    const perpMarket = Array.from(group.perpMarketsMapByName.values()).filter(
      (pm) => pm.name == 'SOL-PERP',
    )[0];

    console.log(
      `Long funding ${perpMarket.longFunding.toNumber().toLocaleString()}`,
    );
    console.log(
      `Short funding ${perpMarket.shortFunding.toNumber().toLocaleString()}`,
    );
    const bids = await perpMarket.loadBids(client);
    const asks = await perpMarket.loadAsks(client);
    console.log(`FR ${perpMarket.getInstantaneousFundingRateUi(bids, asks)}`);

    const mangoAccount = await client.getMangoAccount(
      new PublicKey('BLgb4NFwhpurMrGX5LQfb8D8dBpGSGtBqqew2Em8uyRT'),
      false,
    );
    const perpPosition = mangoAccount.getPerpPosition(
      perpMarket.perpMarketIndex,
    );
    console.log(
      `Long settled funding ${perpPosition?.longSettledFunding
        .toNumber()
        .toLocaleString()}`,
    );
    console.log(
      `Short settled funding ${perpPosition?.shortSettledFunding
        .toNumber()
        .toLocaleString()}`,
    );
    console.log(
      `Unsettled funding ui ${perpPosition?.getUnsettledFundingUi(perpMarket)}`,
    );
    console.log('');
    await new Promise((r) => setTimeout(r, 5 * 1000));
  }
}

main();
