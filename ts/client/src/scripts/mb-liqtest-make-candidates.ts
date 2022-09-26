import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// This script creates liquidation candidates
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 200);

// native prices
const PRICES = {
  BTC: 20000.0,
  SOL: 0.04,
  USDC: 1,
};

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
]);

const TOKEN_SCENARIOS: [string, string, number, string, number][] = [
  ['LIQTEST, LIQOR', 'USDC', 1000000, 'USDC', 0],
  ['LIQTEST, A: USDC, L: SOL', 'USDC', 1000 * PRICES.SOL, 'SOL', 920],
  ['LIQTEST, A: SOL, L: USDC', 'SOL', 1000, 'USDC', 920 * PRICES.SOL],
  ['LIQTEST, A: BTC, L: SOL', 'BTC', 20, 'SOL', (18 * PRICES.BTC) / PRICES.SOL],
];

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(admin);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {},
    'get-program-accounts',
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  const accounts = await client.getMangoAccountsForOwner(
    group,
    admin.publicKey,
  );
  let maxAccountNum = Math.max(0, ...accounts.map((a) => a.accountNum));

  for (const scenario of TOKEN_SCENARIOS) {
    const [name, assetName, assetAmount, liabName, liabAmount] = scenario;

    // create account
    console.log(`Creating mangoaccount...`);
    let mangoAccount = (await client.getOrCreateMangoAccount(
      group,
      admin.publicKey,
      maxAccountNum + 1,
      name,
    ))!;
    maxAccountNum = maxAccountNum + 1;
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const assetMint = new PublicKey(MAINNET_MINTS.get(assetName)!);
    const liabMint = new PublicKey(MAINNET_MINTS.get(liabName)!);

    await client.tokenDepositNative(
      group,
      mangoAccount,
      assetMint,
      assetAmount,
    );
    await mangoAccount.reload(client, group);

    if (liabAmount > 0) {
      // temporarily drop the borrowed token value, so the borrow goes through
      const oracle = group.banksMapByName.get(liabName)![0].oracle;
      try {
        await client.stubOracleSet(group, oracle, PRICES[liabName] / 2);

        await client.tokenWithdrawNative(
          group,
          mangoAccount,
          liabMint,
          liabAmount,
          true,
        );
      } finally {
        // restore the oracle
        await client.stubOracleSet(group, oracle, PRICES[liabName]);
      }
    }
  }

  // Serum order scenario
  {
    const name = 'LIQTEST, serum orders';

    console.log(`Creating mangoaccount...`);
    let mangoAccount = (await client.getOrCreateMangoAccount(
      group,
      admin.publicKey,
      maxAccountNum + 1,
      name,
    ))!;
    maxAccountNum = maxAccountNum + 1;
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const market = group.getSerum3MarketByIndexByName('SOL/USDC')!;
    const sellMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
    const buyMint = new PublicKey(MAINNET_MINTS.get('SOL')!);

    await client.tokenDepositNative(group, mangoAccount, sellMint, 100000);
    await mangoAccount.reload(client, group);

    // temporarily up the init asset weight of the bought token
    await client.tokenEdit(
      group,
      buyMint,
      null,
      null,
      null,
      null,
      null,
      null,
      1.0,
      1.0,
      null,
      null,
      null,
    );
    try {
      // At a price of $1/ui-SOL we can buy 0.1 ui-SOL for the 100k native-USDC we have.
      // With maint weight of 0.9 we have 10x main-leverage. Buying 12x as much causes liquidation.
      await client.serum3PlaceOrder(
        group,
        mangoAccount,
        market.serumMarketExternal,
        Serum3Side.bid,
        1,
        12 * 0.1,
        Serum3SelfTradeBehavior.abortTransaction,
        Serum3OrderType.limit,
        0,
        5,
      );
    } finally {
      // restore the weights
      await client.tokenEdit(
        group,
        buyMint,
        null,
        null,
        null,
        null,
        null,
        null,
        0.9,
        0.8,
        null,
        null,
        null,
      );
    }
  }

  process.exit();
}

main();
