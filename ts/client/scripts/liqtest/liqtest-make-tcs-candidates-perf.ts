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
  ['TCS, LIQOR', [['USDC', 100000000]], []],
];

const NUM_ACCOUNTS = 4;
const NUM_TCS_PER_ACCOUNT = 20;

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

  for (let i = 0; i < NUM_ACCOUNTS; i++) {
    const name = `TCS, A${i}`;

    console.log(`Creating mangoaccount...`);
    const mangoAccount = await createMangoAccount(name);
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    const assetMint = new PublicKey(MINTS.get('USDC')!);
    await client.tokenDepositNative(
      group,
      mangoAccount,
      assetMint,
      new BN(1000000),
    );

    await mangoAccount.reload(client);

    for (let i = 0; i < NUM_TCS_PER_ACCOUNT; i++) {
      await client.tokenConditionalSwapCreateRaw(
        group,
        mangoAccount,
        MINTS.get('SOL')!,
        MINTS.get('USDC')!,
        new BN(100000000),
        new BN(20000),
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
  }

  process.exit();
}

main();
