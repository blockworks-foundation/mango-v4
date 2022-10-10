import { AnchorProvider, BN, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoAccount } from '../accounts/mangoAccount';
import { PerpOrderSide, PerpOrderType } from '../accounts/perp';
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
  MNGO: 0.04,
};

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['MNGO', 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac'],
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
  const fundingAccount = accounts.find(
    (account) => account.name == 'LIQTEST, FUNDING',
  );
  if (!fundingAccount) {
    throw new Error('could not find funding account');
  }

  async function createMangoAccount(name: string): Promise<MangoAccount> {
    const accountNum = maxAccountNum + 1;
    maxAccountNum = maxAccountNum + 1;
    await client.createMangoAccount(group, accountNum, name, 4, 4, 4, 4);
    return (await client.getMangoAccountForOwner(
      group,
      admin.publicKey,
      accountNum,
    ))!;
  }

  for (const scenario of TOKEN_SCENARIOS) {
    const [name, assetName, assetAmount, liabName, liabAmount] = scenario;

    // create account
    console.log(`Creating mangoaccount...`);
    let mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const assetMint = new PublicKey(MAINNET_MINTS.get(assetName)!);
    const liabMint = new PublicKey(MAINNET_MINTS.get(liabName)!);

    await client.tokenDepositNative(
      group,
      mangoAccount,
      assetMint,
      new BN(assetAmount),
    );
    await mangoAccount.reload(client);

    if (liabAmount > 0) {
      // temporarily drop the borrowed token value, so the borrow goes through
      const oracle = group.banksMapByName.get(liabName)![0].oracle;
      try {
        await client.stubOracleSet(group, oracle, PRICES[liabName] / 2);

        await client.tokenWithdrawNative(
          group,
          mangoAccount,
          liabMint,
          new BN(liabAmount),
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
    let mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const market = group.getSerum3MarketByName('SOL/USDC')!;
    const sellMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
    const buyMint = new PublicKey(MAINNET_MINTS.get('SOL')!);

    await client.tokenDepositNative(
      group,
      mangoAccount,
      sellMint,
      new BN(100000),
    );
    await mangoAccount.reload(client);

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

  // Perp orders bring health <0, liquidator force closes
  {
    const name = 'LIQTEST, perp orders';

    console.log(`Creating mangoaccount...`);
    let mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const baseMint = new PublicKey(MAINNET_MINTS.get('MNGO')!);
    const collateralMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
    const collateralOracle = group.banksMapByName.get('SOL')![0].oracle;

    await client.tokenDepositNative(
      group,
      mangoAccount,
      collateralMint,
      new BN(100000),
    ); // valued as $0.004 maint collateral
    await mangoAccount.reload(client);

    await client.stubOracleSet(group, collateralOracle, PRICES['SOL'] * 4);

    try {
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
        PerpOrderSide.bid,
        1, // ui price that won't get hit
        0.0011, // ui base quantity, 11 base lots, $0.044
        0.044, // ui quote quantity
        4200,
        PerpOrderType.limit,
        0,
        5,
      );
    } finally {
      await client.stubOracleSet(group, collateralOracle, PRICES['SOL']);
    }
  }

  // Perp base pos brings health<0, liquidator takes most of it
  {
    const name = 'LIQTEST, perp base pos';

    console.log(`Creating mangoaccount...`);
    let mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const baseMint = new PublicKey(MAINNET_MINTS.get('MNGO')!);
    const collateralMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
    const collateralOracle = group.banksMapByName.get('SOL')![0].oracle;

    await client.tokenDepositNative(
      group,
      mangoAccount,
      collateralMint,
      new BN(100000),
    ); // valued as $0.004 maint collateral
    await mangoAccount.reload(client);

    await client.stubOracleSet(group, collateralOracle, PRICES['SOL'] * 5);

    try {
      await client.perpPlaceOrder(
        group,
        fundingAccount,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
        PerpOrderSide.ask,
        40,
        0.0011, // ui base quantity, 11 base lots, $0.044
        0.044, // ui quote quantity
        4200,
        PerpOrderType.limit,
        0,
        5,
      );

      await client.perpPlaceOrder(
        group,
        mangoAccount,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
        PerpOrderSide.bid,
        40,
        0.0011, // ui base quantity, 11 base lots, $0.044
        0.044, // ui quote quantity
        4200,
        PerpOrderType.market,
        0,
        5,
      );

      await client.perpConsumeAllEvents(
        group,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
      );
    } finally {
      await client.stubOracleSet(group, collateralOracle, PRICES['SOL']);
    }
  }

  // borrows and positive perp pnl (but no position)
  {
    const name = 'LIQTEST, perp positive pnl';

    console.log(`Creating mangoaccount...`);
    let mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const baseMint = new PublicKey(MAINNET_MINTS.get('MNGO')!);
    const baseOracle = (await client.getStubOracle(group, baseMint))[0]
      .publicKey;
    const liabMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
    const collateralMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
    const collateralOracle = group.banksMapByName.get('SOL')![0].oracle;

    await client.tokenDepositNative(
      group,
      mangoAccount,
      collateralMint,
      new BN(100000),
    ); // valued as $0.004 maint collateral
    await mangoAccount.reload(client);

    try {
      await client.stubOracleSet(group, collateralOracle, PRICES['SOL'] * 10);

      // Spot-borrow more than the collateral is worth
      await client.tokenWithdrawNative(
        group,
        mangoAccount,
        liabMint,
        new BN(-5000),
        true,
      );
      await mangoAccount.reload(client);

      // Execute two trades that leave the account with +$0.022 positive pnl
      await client.stubOracleSet(group, baseOracle, PRICES['MNGO'] / 2);
      await client.perpPlaceOrder(
        group,
        fundingAccount,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
        PerpOrderSide.ask,
        20,
        0.0011, // ui base quantity, 11 base lots, $0.022
        0.022, // ui quote quantity
        4200,
        PerpOrderType.limit,
        0,
        5,
      );
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
        PerpOrderSide.bid,
        20,
        0.0011, // ui base quantity, 11 base lots, $0.022
        0.022, // ui quote quantity
        4200,
        PerpOrderType.market,
        0,
        5,
      );
      await client.perpConsumeAllEvents(
        group,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
      );

      await client.stubOracleSet(group, baseOracle, PRICES['MNGO']);

      await client.perpPlaceOrder(
        group,
        fundingAccount,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
        PerpOrderSide.bid,
        40,
        0.0011, // ui base quantity, 11 base lots, $0.044
        0.044, // ui quote quantity
        4201,
        PerpOrderType.limit,
        0,
        5,
      );
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
        PerpOrderSide.ask,
        40,
        0.0011, // ui base quantity, 11 base lots, $0.044
        0.044, // ui quote quantity
        4201,
        PerpOrderType.market,
        0,
        5,
      );
      await client.perpConsumeAllEvents(
        group,
        group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex!,
      );
    } finally {
      await client.stubOracleSet(group, collateralOracle, PRICES['SOL']);
      await client.stubOracleSet(group, baseOracle, PRICES['MNGO']);
    }
  }

  process.exit();
}

main();
