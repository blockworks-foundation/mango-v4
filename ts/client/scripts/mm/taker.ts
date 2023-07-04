import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../../src/accounts/group';
import { MangoAccount } from '../../src/accounts/mangoAccount';
import {
  PerpMarket,
  PerpOrderSide,
  PerpOrderType,
} from '../../src/accounts/perp';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { ZERO_I80F48 } from '../../src/numbers/I80F48';
import { toNativeI80F48, toUiDecimalsForQuote } from '../../src/utils';

// For easy switching between mainnet and devnet, default is mainnet
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';

async function settlePnl(
  mangoAccount: MangoAccount,
  perpMarket: PerpMarket,
  client: MangoClient,
  group: Group,
) {
  if (!mangoAccount.perpPositionExistsForMarket(perpMarket)) {
    return;
  }

  const pp = mangoAccount
    .perpActive()
    .find((pp) => pp.marketIndex === perpMarket.perpMarketIndex)!;
  const pnl = pp.getUnsettledPnl(perpMarket);

  console.log(
    `Avg entry price - ${pp.getAverageEntryPriceUi(
      perpMarket,
    )}, Breakeven price - ${pp.getBreakEvenPriceUi(perpMarket)}`,
  );

  let profitableAccount, unprofitableAccount;

  if (pnl.abs().gt(toNativeI80F48(1, 6))) {
    console.log(`- Settling pnl ${toUiDecimalsForQuote(pnl)} ...`);
  } else {
    console.log(
      `- Skipping Settling pnl ${toUiDecimalsForQuote(pnl)}, too small`,
    );
    return;
  }

  if (pnl.gt(ZERO_I80F48())) {
    console.log(`- Settling profit pnl...`);
    profitableAccount = mangoAccount;
    const candidates = await perpMarket.getSettlePnlCandidates(
      client,
      group,
      undefined,
      'negative',
    );
    if (candidates.length === 0) {
      return;
    }
    unprofitableAccount = candidates[0].account;
    const sig = await client.perpSettlePnl(
      group,
      profitableAccount,
      unprofitableAccount,
      mangoAccount,
      perpMarket.perpMarketIndex,
    );
    console.log(
      `- Settled pnl, sig https://explorer.solana.com/tx/${sig}?cluster=${
        CLUSTER == 'devnet' ? 'devnet' : ''
      }`,
    );
  } else if (pnl.lt(ZERO_I80F48())) {
    unprofitableAccount = mangoAccount;
    const candidates = await perpMarket.getSettlePnlCandidates(
      client,
      group,
      undefined,
      'positive',
    );
    if (candidates.length === 0) {
      return;
    }
    profitableAccount = candidates[0].account;
    console.log(`- Settling loss pnl...`);
    let sig = await client.perpSettlePnl(
      group,
      profitableAccount,
      unprofitableAccount,
      mangoAccount,
      perpMarket.perpMarketIndex,
    );
    console.log(
      `- Settled pnl, sig https://explorer.solana.com/tx/${sig}?cluster=${
        CLUSTER == 'devnet' ? 'devnet' : ''
      }`,
    );
  }
}

async function takeOrder(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
  perpMarket: PerpMarket,
  side: PerpOrderSide,
) {
  await mangoAccount.reload(client);

  const size = Math.random() * 0.001;
  const price =
    side === PerpOrderSide.bid
      ? perpMarket.uiPrice * 1.01
      : perpMarket.uiPrice * 0.99;
  console.log(
    `- ${perpMarket.name} taking with a ${
      side === PerpOrderSide.bid ? 'bid' : 'ask'
    } at  price ${price.toFixed(4)} and size ${size.toFixed(6)}`,
  );

  const oldPosition = mangoAccount.getPerpPosition(perpMarket.perpMarketIndex);
  if (oldPosition) {
    console.log(
      `-- before base: ${perpMarket.baseLotsToUi(
        oldPosition.basePositionLots,
      )}, quote: ${toUiDecimalsForQuote(oldPosition.quotePositionNative)}`,
    );
  }

  await client.perpPlaceOrder(
    group,
    mangoAccount,
    perpMarket.perpMarketIndex,
    side,
    price,
    size,
    undefined,
    Date.now(),
    PerpOrderType.market,
    false,
    0,
    10,
  );

  // Sleep to see change, alternatively we could reload account with processed commitmment
  await new Promise((r) => setTimeout(r, 5000));
  await mangoAccount.reload(client);
  const newPosition = mangoAccount.getPerpPosition(perpMarket.perpMarketIndex);
  if (newPosition) {
    console.log(
      `-- after base: ${perpMarket.baseLotsToUi(
        newPosition.basePositionLots,
      )}, quote: ${toUiDecimalsForQuote(newPosition.quotePositionNative)}`,
    );
  }
}

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(USER_KEYPAIR!, 'utf-8'))),
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

  // Load mango account
  let mangoAccount = await client.getMangoAccount(
    new PublicKey(MANGO_ACCOUNT_PK),
  );
  await mangoAccount.reload(client);

  // Load group
  const group = await client.getGroup(mangoAccount.group);
  await group.reloadAll(client);

  // Take on OB
  const perpMarket = group.getPerpMarketByName('BTC-PERP');
  while (true) {
    await group.reloadAll(client);

    // Settle pnl
    await settlePnl(mangoAccount, perpMarket, client, group);

    await takeOrder(client, group, mangoAccount, perpMarket, PerpOrderSide.bid);
    await takeOrder(client, group, mangoAccount, perpMarket, PerpOrderSide.ask);
  }
}

main();
