import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

//
// This script tries to withdraw all positive balances for all accounts
// by PAYER_KEYPAIR in the group.
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 200);
const CLUSTER = process.env.CLUSTER || 'mainnet-beta';
const CLUSTER_URL = process.env.CLUSTER_URL;
const PAYER_KEYPAIR = process.env.PAYER_KEYPAIR || '';

async function main() {
  const options = AnchorProvider.defaultOptions();
  options.commitment = 'processed';
  options.preflightCommitment = 'finalized';
  const connection = new Connection(CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(PAYER_KEYPAIR, 'utf-8'))),
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

  // const groups = await client.getGroupsForCreator(admin.publicKey);
  // console.log(
  //   'groups: ',
  //   groups.map((g) => g.publicKey.toString() + '  ' + g.groupNum.toString()),
  // );

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  let accounts = await client.getMangoAccountsForOwner(group, admin.publicKey);
  for (const account of accounts) {
    for (const serumOrders of account.serum3Active()) {
      const serumMarket = group.getSerum3MarketByMarketIndex(
        serumOrders.marketIndex,
      )!;
      const serumExternal = serumMarket.serumMarketExternal;
      console.log(
        `closing serum orders on: ${account} for market ${serumMarket.name}`,
      );
      await client.serum3CancelAllOrders(group, account, serumExternal, 10);
      try {
        await client.serum3ConsumeEvents(group, serumExternal);
      } catch (e) {}
      await client.serum3SettleFunds(group, account, serumExternal);
      await client.serum3CloseOpenOrders(group, account, serumExternal);
    }

    for (const perpPosition of account.perpActive()) {
      const perpMarket = group.findPerpMarket(perpPosition.marketIndex)!;
      console.log(
        `closing perp orders on: ${account} for market ${perpMarket.name}`,
      );
      await client.perpCancelAllOrders(
        group,
        account,
        perpMarket.perpMarketIndex,
        10,
      );
    }
  }

  accounts = await client.getMangoAccountsForOwner(group, admin.publicKey);
  for (const account of accounts) {
    // close account
    try {
      console.log(`closing account: ${account}`);
      await client.closeMangoAccount(group, account, true);
    } catch (error) {
      console.log(`failed to close ${account.publicKey}: ${error}`);
    }
  }

  process.exit();
}

main();
