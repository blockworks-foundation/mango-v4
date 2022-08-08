import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// This script creates liquidation candidates
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 1);

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

const SCENARIOS: [string, string, number, string, number][] = [
  ['LIQTEST, A: USDC, L: SOL', 'USDC', 1000 * PRICES.SOL, 'SOL', 999],
  //['LIQTEST, A: SOL, L: USDC', 'SOL', 1000, 'USDC', 999 * PRICES.SOL],
  //['LIQTEST, A: BTC, L: SOL', 'BTC', 20, 'SOL', 19 * PRICES.BTC / PRICES.SOL],
];

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL, options);

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
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  const accounts = await client.getMangoAccountsForOwner(
    group,
    admin.publicKey,
  );
  let maxAccountNum = Math.max(...accounts.map((a) => a.accountNum));
  console.log(maxAccountNum);

  for (const scenario of SCENARIOS) {
    const [name, assetName, assetAmount, liabName, liabAmount] = scenario;

    // create account
    console.log(`Creating mangoaccount...`);
    let mangoAccount = await client.getOrCreateMangoAccount(
      group,
      admin.publicKey,
      maxAccountNum + 1,
    );
    maxAccountNum = maxAccountNum + 1;
    console.log(
      `...created mangoAccount ${mangoAccount.publicKey} for ${name}`,
    );

    await client.tokenDepositNative(
      group,
      mangoAccount,
      assetName,
      assetAmount,
    );
    await mangoAccount.reload(client, group);

    // temporarily drop the borrowed token value, so the borrow goes through
    const oracle = group.banksMap.get(liabName).oracle;
    await client.stubOracleSet(group, oracle, PRICES[liabName] / 2);

    await client.tokenWithdrawNative(
      group,
      mangoAccount,
      liabName,
      liabAmount,
      true,
    );

    // restore the oracle
    await client.stubOracleSet(group, oracle, PRICES[liabName]);
  }

  process.exit();
}

main();
