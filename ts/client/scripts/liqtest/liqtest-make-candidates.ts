import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Bank } from '../../src/accounts/bank';
import { MangoAccount } from '../../src/accounts/mangoAccount';
import {
  PerpMarket,
  PerpOrderSide,
  PerpOrderType,
} from '../../src/accounts/perp';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../../src/accounts/serum3';
import { Builder } from '../../src/builder';
import { MangoClient } from '../../src/client';
import {
  NullPerpEditParams,
  NullTokenEditParams,
} from '../../src/clientIxParamBuilder';
import { MANGO_V4_ID } from '../../src/constants';

//
// This script creates liquidation candidates
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 200);
const CLUSTER = process.env.CLUSTER || 'mainnet-beta';

// native prices
const PRICES = {
  ETH: 1200.0,
  SOL: 0.015, // not updated for the fact that the new mints we use have 6 decimals!
  USDC: 1,
  MNGO: 0.02,
};

const TOKEN_SCENARIOS: [string, [string, number][], [string, number][]][] = [
  [
    'LIQTEST, FUNDING',
    [
      ['USDC', 5000000],
      ['ETH', 100000],
      ['SOL', 150000000],
    ],
    [],
  ],
  ['LIQTEST, LIQOR', [['USDC', 1000000]], []],
  ['LIQTEST, A: USDC, L: SOL', [['USDC', 1000 * PRICES.SOL]], [['SOL', 920]]],
  ['LIQTEST, A: SOL, L: USDC', [['SOL', 1000]], [['USDC', 990 * PRICES.SOL]]],
  [
    'LIQTEST, A: ETH, L: SOL',
    [['ETH', 20]],
    [['SOL', (18 * PRICES.ETH) / PRICES.SOL]],
  ],
];

async function main() {
  const options = AnchorProvider.defaultOptions();
  options.commitment = 'processed';
  options.preflightCommitment = 'finalized';
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(admin);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER as Cluster,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
      prioritizationFee: 100,
      txConfirmationCommitment: 'confirmed',
    },
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  const MAINNET_MINTS = new Map([
    ['USDC', group.banksMapByName.get('USDC')![0].mint],
    ['ETH', group.banksMapByName.get('ETH')![0].mint],
    ['SOL', group.banksMapByName.get('SOL')![0].mint],
  ]);

  const accounts = await client.getMangoAccountsForOwner(
    group,
    admin.publicKey,
  );
  let maxAccountNum = Math.max(0, ...accounts.map((a) => a.accountNum));

  async function createMangoAccount(name: string): Promise<MangoAccount> {
    const accountNum = maxAccountNum + 1;
    maxAccountNum = maxAccountNum + 1;
    await client.createMangoAccount(group, accountNum, name, 5, 4, 4, 4);
    return (await client.getMangoAccountForOwner(
      group,
      admin.publicKey,
      accountNum,
    ))!;
  }

  async function setBankPrice(bank: Bank, price: number): Promise<void> {
    await client.stubOracleSet(group, bank.oracle, price);
    // reset stable price
    await client.tokenEdit(
      group,
      bank.mint,
      Builder(NullTokenEditParams).resetStablePrice(true).build(),
    );
  }
  async function setPerpPrice(
    perpMarket: PerpMarket,
    price: number,
  ): Promise<void> {
    await client.stubOracleSet(group, perpMarket.oracle, price);
    // reset stable price
    await client.perpEditMarket(
      group,
      perpMarket.perpMarketIndex,
      Builder(NullPerpEditParams).resetStablePrice(true).build(),
    );
  }

  for (const scenario of TOKEN_SCENARIOS) {
    const [name, assets, liabs] = scenario;

    // create account
    console.log(`Creating mangoaccount...`);
    const mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    for (const [assetName, assetAmount] of assets) {
      const assetMint = new PublicKey(MAINNET_MINTS.get(assetName)!);
      await client.tokenDepositNative(
        group,
        mangoAccount,
        assetMint,
        new BN(assetAmount),
      );
      await mangoAccount.reload(client);
    }

    for (const [liabName, liabAmount] of liabs) {
      const liabMint = new PublicKey(MAINNET_MINTS.get(liabName)!);

      // temporarily drop the borrowed token value, so the borrow goes through
      const bank = group.banksMapByName.get(liabName)![0];
      try {
        await setBankPrice(bank, PRICES[liabName] / 2);

        await client.tokenWithdrawNative(
          group,
          mangoAccount,
          liabMint,
          new BN(liabAmount),
          true,
        );
      } finally {
        // restore the oracle
        await setBankPrice(bank, PRICES[liabName]);
      }
    }
  }

  const accounts2 = await client.getMangoAccountsForOwner(
    group,
    admin.publicKey,
  );
  const fundingAccount = accounts2.find(
    (account) => account.name == 'LIQTEST, FUNDING',
  );
  if (!fundingAccount) {
    throw new Error('could not find funding account');
  }

  // Serum order scenario
  {
    const name = 'LIQTEST, serum orders';

    console.log(`Creating mangoaccount...`);
    const mangoAccount = await createMangoAccount(name);
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
      new BN(150000),
    );
    await mangoAccount.reload(client);

    // temporarily up the init asset weight of the bought token
    await client.tokenEdit(
      group,
      buyMint,
      Builder(NullTokenEditParams)
        .oracle(group.getFirstBankByMint(buyMint).oracle)
        .maintAssetWeight(1.0)
        .initAssetWeight(1.0)
        .build(),
    );
    try {
      // At a price of $0.015/ui-SOL we can buy 10 ui-SOL for the 0.15 USDC (150k native-USDC) we have.
      // With maint weight of 0.9 we have 10x main-leverage. Buying 11x as much causes liquidation.
      await client.serum3PlaceOrder(
        group,
        mangoAccount,
        market.serumMarketExternal,
        Serum3Side.bid,
        0.015,
        11 * 10,
        Serum3SelfTradeBehavior.abortTransaction,
        Serum3OrderType.limit,
        0,
        5,
      );
      await mangoAccount.reload(client);

      for (let market of group.serum3MarketsMapByMarketIndex.values()) {
        if (market.name == 'SOL/USDC') {
          continue;
        }
        await client.serum3PlaceOrder(
          group,
          mangoAccount,
          market.serumMarketExternal,
          Serum3Side.bid,
          0.001,
          1,
          Serum3SelfTradeBehavior.abortTransaction,
          Serum3OrderType.limit,
          0,
          5,
        );
        await mangoAccount.reload(client);
      }
    } finally {
      // restore the weights
      await client.tokenEdit(
        group,
        buyMint,
        Builder(NullTokenEditParams)
          .oracle(group.getFirstBankByMint(buyMint).oracle)
          .maintAssetWeight(0.9)
          .initAssetWeight(0.8)
          .build(),
      );
    }
  }

  // Perp orders bring health <0, liquidator force closes
  {
    const name = 'LIQTEST, perp orders';

    console.log(`Creating mangoaccount...`);
    const mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const collateralMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
    const collateralBank = group.banksMapByName.get('SOL')![0];

    await client.tokenDepositNative(
      group,
      mangoAccount,
      collateralMint,
      new BN(300000),
    ); // valued as 0.0003 SOL, $0.0045 maint collateral
    await mangoAccount.reload(client);

    await setBankPrice(collateralBank, PRICES['SOL'] * 4);

    try {
      // placing this order decreases maint health by (0.9 - 1)*$0.06 = $-0.006
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        assertNotUndefined(
          group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex,
        ),
        PerpOrderSide.bid,
        0.001, // ui price that won't get hit
        3.0, // ui base quantity, 30 base lots, 3.0 MNGO, $0.06
        0.06, // ui quote quantity
        4200,
        PerpOrderType.limit,
        false,
        0,
        5,
      );
    } finally {
      await setBankPrice(collateralBank, PRICES['SOL']);
    }
  }

  // Perp base pos brings health<0, liquidator takes most of it
  {
    const name = 'LIQTEST, perp base pos';

    console.log(`Creating mangoaccount...`);
    const mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const collateralMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
    const collateralBank = group.banksMapByName.get('SOL')![0];

    await client.tokenDepositNative(
      group,
      mangoAccount,
      collateralMint,
      new BN(300000),
    ); // valued as 0.0003 SOL, $0.0045 maint collateral
    await mangoAccount.reload(client);

    await setBankPrice(collateralBank, PRICES['SOL'] * 10);

    try {
      await client.perpPlaceOrder(
        group,
        fundingAccount,
        assertNotUndefined(
          group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex,
        ),
        PerpOrderSide.ask,
        0.03,
        1.1, // ui base quantity, 11 base lots, $0.022 value, gain $0.033
        0.033, // ui quote quantity
        4200,
        PerpOrderType.limit,
        false,
        0,
        5,
      );

      await client.perpPlaceOrder(
        group,
        mangoAccount,
        assertNotUndefined(
          group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex,
        ),
        PerpOrderSide.bid,
        0.03,
        1.1, // ui base quantity, 11 base lots, $0.022 value, cost $0.033
        0.033, // ui quote quantity
        4200,
        PerpOrderType.market,
        false,
        0,
        5,
      );

      await client.perpConsumeAllEvents(
        group,
        assertNotUndefined(
          group.perpMarketsMapByName.get('MNGO-PERP')?.perpMarketIndex,
        ),
      );
    } finally {
      await setBankPrice(collateralBank, PRICES['SOL']);
    }
  }

  // borrows and positive perp pnl (but no position)
  {
    const name = 'LIQTEST, perp positive pnl';

    console.log(`Creating mangoaccount...`);
    const mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const perpMarket = group.perpMarketsMapByName.get('MNGO-PERP')!;
    const perpIndex = perpMarket.perpMarketIndex;
    const liabMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
    const collateralMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
    const collateralBank = group.banksMapByName.get('SOL')![0];

    await client.tokenDepositNative(
      group,
      mangoAccount,
      collateralMint,
      new BN(300000),
    ); // valued as $0.0045 maint collateral
    await mangoAccount.reload(client);

    try {
      await setBankPrice(collateralBank, PRICES['SOL'] * 10);

      // Spot-borrow more than the collateral is worth
      await client.tokenWithdrawNative(
        group,
        mangoAccount,
        liabMint,
        new BN(-5000),
        true,
      );
      await mangoAccount.reload(client);

      // Execute two trades that leave the account with +$0.011 positive pnl
      await setPerpPrice(perpMarket, PRICES['MNGO'] / 2);
      await client.perpPlaceOrder(
        group,
        fundingAccount,
        perpIndex,
        PerpOrderSide.ask,
        0.01,
        1.1, // ui base quantity, 11 base lots, $0.011
        0.011, // ui quote quantity
        4200,
        PerpOrderType.limit,
        false,
        0,
        5,
      );
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpIndex,
        PerpOrderSide.bid,
        0.01,
        1.1, // ui base quantity, 11 base lots, $0.011
        0.011, // ui quote quantity
        4200,
        PerpOrderType.market,
        false,
        0,
        5,
      );
      await client.perpConsumeAllEvents(group, perpIndex);

      await setPerpPrice(perpMarket, PRICES['MNGO']);

      await client.perpPlaceOrder(
        group,
        fundingAccount,
        perpIndex,
        PerpOrderSide.bid,
        0.02,
        1.1, // ui base quantity, 11 base lots, $0.022
        0.022, // ui quote quantity
        4201,
        PerpOrderType.limit,
        false,
        0,
        5,
      );
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpIndex,
        PerpOrderSide.ask,
        0.02,
        1.1, // ui base quantity, 11 base lots, $0.022
        0.022, // ui quote quantity
        4201,
        PerpOrderType.market,
        false,
        0,
        5,
      );
      await client.perpConsumeAllEvents(group, perpIndex);
    } finally {
      await setPerpPrice(perpMarket, PRICES['MNGO']);
      await setBankPrice(collateralBank, PRICES['SOL']);
    }
  }

  // assets and negative perp pnl (but no position)
  {
    const name = 'LIQTEST, perp negative pnl';

    console.log(`Creating mangoaccount...`);
    const mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const perpMarket = group.perpMarketsMapByName.get('MNGO-PERP')!;
    const perpIndex = perpMarket.perpMarketIndex;
    const liabMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
    const collateralMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
    const collateralBank = group.banksMapByName.get('SOL')![0];

    await client.tokenDepositNative(
      group,
      mangoAccount,
      collateralMint,
      new BN(300000),
    ); // valued as $0.0045 maint collateral
    await mangoAccount.reload(client);

    try {
      await setBankPrice(collateralBank, PRICES['SOL'] * 10);

      // Execute two trades that leave the account with -$0.011 negative pnl
      await setPerpPrice(perpMarket, PRICES['MNGO'] / 2);
      await client.perpPlaceOrder(
        group,
        fundingAccount,
        perpIndex,
        PerpOrderSide.bid,
        0.01,
        1.1, // ui base quantity, 11 base lots, $0.011
        0.011, // ui quote quantity
        4200,
        PerpOrderType.limit,
        false,
        0,
        5,
      );
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpIndex,
        PerpOrderSide.ask,
        0.01,
        1.1, // ui base quantity, 11 base lots, $0.011
        0.011, // ui quote quantity
        4200,
        PerpOrderType.market,
        false,
        0,
        5,
      );
      await client.perpConsumeAllEvents(group, perpIndex);

      await setPerpPrice(perpMarket, PRICES['MNGO']);

      await client.perpPlaceOrder(
        group,
        fundingAccount,
        perpIndex,
        PerpOrderSide.ask,
        0.02,
        1.1, // ui base quantity, 11 base lots, $0.022
        0.022, // ui quote quantity
        4201,
        PerpOrderType.limit,
        false,
        0,
        5,
      );
      await client.perpPlaceOrder(
        group,
        mangoAccount,
        perpIndex,
        PerpOrderSide.bid,
        0.02,
        1.1, // ui base quantity, 11 base lots, $0.022
        0.022, // ui quote quantity
        4201,
        PerpOrderType.market,
        false,
        0,
        5,
      );
      await client.perpConsumeAllEvents(group, perpIndex);
    } finally {
      await setPerpPrice(perpMarket, PRICES['MNGO']);
      await setBankPrice(collateralBank, PRICES['SOL']);
    }
  }

  process.exit();
}

function assertNotUndefined<T>(value: T | undefined): T {
  if (value === undefined) {
    throw new Error('Value was undefined');
  }
  return value;
}

main();
