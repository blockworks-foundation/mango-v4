import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../../accounts/group';
import { MangoAccount } from '../../accounts/mangoAccount';
import { PerpMarket, PerpOrderSide, PerpOrderType } from '../../accounts/perp';
import { MangoClient } from '../../client';
import { MANGO_V4_ID } from '../../constants';
import { toUiDecimalsForQuote } from '../../utils';

// For easy switching between mainnet and devnet, default is mainnet
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';

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
    `${perpMarket.name} taking with a ${
      side === PerpOrderSide.bid ? 'bid' : 'ask'
    } at  price ${price.toFixed(4)} and size ${size.toFixed(6)}`,
  );

  const oldPosition = mangoAccount.getPerpPosition(perpMarket.perpMarketIndex);
  if (oldPosition) {
    console.log(
      `- before base: ${perpMarket.baseLotsToUi(
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
      `- after base: ${perpMarket.baseLotsToUi(
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
  let mangoAccount = await client.getMangoAccountForPublicKey(
    new PublicKey(MANGO_ACCOUNT_PK),
  );
  await mangoAccount.reload(client);

  // Load group
  const group = await client.getGroup(mangoAccount.group);
  await group.reloadAll(client);

  // Take on OB
  const perpMarket = group.getPerpMarketByName('BTC-PERP');
  while (true) {
    await takeOrder(client, group, mangoAccount, perpMarket, PerpOrderSide.bid);
    await takeOrder(client, group, mangoAccount, perpMarket, PerpOrderSide.ask);
  }
}

main();
