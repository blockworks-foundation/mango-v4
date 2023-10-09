import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as splToken from '@solana/spl-token';
import fs from 'fs';
import { Bank } from '../../src/accounts/bank';
import {
  MangoAccount,
  TokenConditionalSwapDisplayPriceStyle,
  TokenConditionalSwapIntention,
} from '../../src/accounts/mangoAccount';
import { PerpMarket } from '../../src/accounts/perp';
import { Builder } from '../../src/builder';
import { MangoClient } from '../../src/client';
import {
  DefaultTokenRegisterParams,
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
  SOL: 0.015,
  USDC: 1,
  MNGO: 0.02,
};

const TOKEN_SCENARIOS: [string, [string, number][], [string, number][]][] = [
  [
    'TCS, FUNDING',
    [
      ['USDC', 5000000],
      ['ETH', 100000],
      ['SOL', 150000000],
    ],
    [],
  ],
  ['TCS, LIQOR', [['USDC', 1000000]], []],
  ['TCS, HEALTH', [['USDC', 1000]], []],
  ['TCS, FULL+CLOSE', [['USDC', 1000000]], []],
  ['TCS, EXPIRE', [['USDC', 1000000]], []],
  ['TCS, NET-BORROW', [['USDC', 10000000]], []],
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

  const MINTS = new Map([
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
    await client.createMangoAccount(group, accountNum, name, 4, 4, 4, 4);
    const account = (await client.getMangoAccountForOwner(
      group,
      admin.publicKey,
      accountNum,
    ))!;
    await client.accountExpandV2(group, account, 4, 4, 4, 4, 50);
    await account.reload(client);
    return account;
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
      const assetMint = new PublicKey(MINTS.get(assetName)!);
      await client.tokenDepositNative(
        group,
        mangoAccount,
        assetMint,
        new BN(assetAmount),
      );
      await mangoAccount.reload(client);
    }

    for (const [liabName, liabAmount] of liabs) {
      const liabMint = new PublicKey(MINTS.get(liabName)!);

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

  // Case LIQEE1: The liqee does not have enough health for the tcs
  {
    const account = ensure(
      accounts2.find((account) => account.name == 'TCS, HEALTH'),
    );
    await client.tokenConditionalSwapCreateRaw(
      group,
      account,
      MINTS.get('SOL')!,
      MINTS.get('USDC')!,
      new BN(100000000),
      new BN(20000), // liqee only has 1k USDC-native, leverage does not go that far!
      null,
      0.0,
      1000000.0,
      0.01,
      true,
      true,
      TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
      TokenConditionalSwapIntention.unknown,
    );
  }

  // Case LIQEE2: Full execution - tcs closes afterward
  {
    const account = ensure(
      accounts2.find((account) => account.name == 'TCS, FULL+CLOSE'),
    );
    await client.tokenConditionalSwapCreateRaw(
      group,
      account,
      MINTS.get('SOL')!,
      MINTS.get('USDC')!,
      new BN(1000),
      new BN(1000),
      null,
      0.0,
      1000000.0,
      0.01,
      true,
      true,
      TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
      TokenConditionalSwapIntention.unknown,
    );
  }

  // Case LIQEE3: Create a tcs that will expire very soon
  {
    const account = ensure(
      accounts2.find((account) => account.name == 'TCS, EXPIRE'),
    );
    await client.tokenConditionalSwapCreateRaw(
      group,
      account,
      MINTS.get('SOL')!,
      MINTS.get('USDC')!,
      new BN(1000),
      new BN(1000),
      Date.now() / 1000 + 15, // expire in 15s
      0.0,
      1000000.0,
      0.01,
      true,
      true,
      TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
      TokenConditionalSwapIntention.unknown,
    );
  }

  // Case LIQEE4: Create a tcs that hits net borrow limits
  {
    const account = ensure(
      accounts2.find((account) => account.name == 'TCS, NET-BORROW'),
    );

    // To do this, first make a new mint and register it as a new token with tight net borrow limits
    const newMint = await splToken.createMint(
      connection,
      admin,
      admin.publicKey,
      null,
      6,
    );
    const tokenAccount = await splToken.createAssociatedTokenAccountIdempotent(
      connection,
      admin,
      newMint,
      admin.publicKey,
    );
    await splToken.mintTo(
      connection,
      admin,
      newMint,
      tokenAccount,
      admin,
      1e15,
    );

    await client.stubOracleCreate(group, newMint, 1.0);
    const newOracle = (await client.getStubOracle(group, newMint))[0];
    const newTokenIndex = Math.max(...group.banksMapByTokenIndex.keys()) + 1;
    await client.tokenRegister(
      group,
      newMint,
      newOracle.publicKey,
      newTokenIndex,
      'TMP',
      {
        ...DefaultTokenRegisterParams,
        loanOriginationFeeRate: 0,
        loanFeeRate: 0,
        initAssetWeight: 1,
        maintAssetWeight: 1,
        initLiabWeight: 1,
        maintLiabWeight: 1,
        liquidationFee: 0,
        netBorrowLimitPerWindowQuote: 1500000, // less than the $2 of the tcs
      },
    );
    await group.reloadAll(client);

    await client.tokenConditionalSwapCreateRaw(
      group,
      account,
      newMint,
      MINTS.get('USDC')!,
      new BN(2000000), // $2
      new BN(2000000),
      null,
      0.0,
      1000000.0,
      0.01,
      true,
      true,
      TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
      TokenConditionalSwapIntention.unknown,
    );
  }

  process.exit();
}

function ensure<T>(value: T | undefined): T {
  if (value == null) {
    throw new Error('Value was nullish');
  }
  return value;
}

main();
