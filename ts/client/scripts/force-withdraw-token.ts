import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import fs from 'fs';
import { MANGO_V4_ID, MangoClient, TokenIndex } from '../src';
import { NATIVE_MINT } from '@solana/spl-token';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_PK =
  process.env.GROUP_PK || '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';
const TOKEN_INDEX = Number(process.env.TOKEN_INDEX) as TokenIndex;

async function forceWithdrawTokens(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const forceWithdrawBank = group.getFirstBankByTokenIndex(TOKEN_INDEX);
  console.log(`${forceWithdrawBank.name} bank`);

  const allMangoAccounts = await client.getAllMangoAccounts(group);
  const mangoAccounts = allMangoAccounts
    .filter((a) => {
      if (forceWithdrawBank.mint.equals(NATIVE_MINT)) {
        // WSOL requires many ATA creations so is very expensive. Limit to accounts with >0.005 SOL
        return a.getTokenDepositsUi(forceWithdrawBank) > 0.005;
      } else {
        return a.getTokenDepositsUi(forceWithdrawBank) > 0;
      }
    })
    .sort(
      (a, b) =>
        b.getTokenDepositsUi(forceWithdrawBank) -
        a.getTokenDepositsUi(forceWithdrawBank),
    );

  console.log(
    `Found ${mangoAccounts.length} mango accounts with ${forceWithdrawBank.name} deposits`,
  );

  const batchedMangoAccounts = batchItems(mangoAccounts, 6);
  const ixBatches: TransactionInstruction[][] = [];
  let i = 0;
  for (const batch of batchedMangoAccounts) {
    const ixs: TransactionInstruction[] = [];
    for (const mangoAccount of batch) {
      console.log(
        `${mangoAccount.publicKey} ${forceWithdrawBank.name} balance ${mangoAccount.getTokenBalanceUi(forceWithdrawBank)}`,
      );

      const mangoAccountsIxs = await client.tokenForceWithdrawIxs(
        group,
        mangoAccount,
        forceWithdrawBank.tokenIndex,
      );
      ixs.push(...mangoAccountsIxs);
    }
    ixBatches.push(ixs);
    if (i % 5 === 4 || i === batchedMangoAccounts.length - 1) {
      try {
        const sigs = await Promise.all(
          ixBatches.map((ixs) =>
            client.sendAndConfirmTransactionForGroup(group, ixs),
          ),
        );
        for (const sig of sigs) {
          console.log(`executed sig - ${sig.signature}`);
        }
      } catch (e) {
        console.error(e);
      }
      ixBatches.length = 0;
    }
    i += 1;
  }

  const groupFresh = await client.getGroup(new PublicKey(GROUP_PK));
  const forceWithdrawBankFresh =
    groupFresh.getFirstBankByTokenIndex(TOKEN_INDEX);
  console.log(
    `Final ${forceWithdrawBankFresh.name} deposits ${forceWithdrawBankFresh.uiDeposits()}`,
  );
}

function batchItems<T>(items: T[], batchSize: number): T[][] {
  const batches: T[][] = [];

  for (let i = 0; i < items.length; i += batchSize) {
    const batch = items.slice(i, i + batchSize);
    batches.push(batch);
  }

  return batches;
}

forceWithdrawTokens();
