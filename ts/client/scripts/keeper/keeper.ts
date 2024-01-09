import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import fs from 'fs';
import chunk from 'lodash/chunk';
import range from 'lodash/range';
import { Group } from '../../src/accounts/group';
import { FillEvent, OutEvent, PerpEventQueue } from '../../src/accounts/perp';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { sendTransaction } from '../../src/utils/rpc';

// Env vars
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';
const INTERVAL_UPDATE_BANKS = Number(process.env.INTERVAL_UPDATE_BANKS || 60);
const INTERVAL_CONSUME_EVENTS = Number(
  process.env.INTERVAL_CONSUME_EVENTS || 60,
);
const INTERVAL_UPDATE_FUNDING = Number(
  process.env.INTERVAL_UPDATE_FUNDING || 60,
);
const INTERVAL_CHECK_NEW_LISTINGS_AND_ABORT = Number(
  process.env.INTERVAL_CHECK_NEW_LISTINGS_AND_ABORT || 120,
);

async function updateBanks(client: MangoClient, group: Group): Promise<void> {
  console.log('Starting updateBanks loop');
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const tokenIndices = Array.from(group.banksMapByTokenIndex.keys());
    const tokenIndicesByChunks = chunk(tokenIndices, 10);
    tokenIndicesByChunks.map(async (tokenIndices) => {
      const ixs: TransactionInstruction[] = [];

      for (const tokenIndex of tokenIndices) {
        const ix = await client.tokenUpdateIndexAndRateIx(
          group,
          group.getFirstBankByTokenIndex(tokenIndex).mint,
        );
        await client.connection
          .simulateTransaction(new Transaction().add(ix))
          .then((d) => ixs.push(ix));
      }

      try {
        const sig = await sendTransaction(
          client.program.provider as AnchorProvider,
          ixs,
          group.addressLookupTablesList,
          { prioritizationFee: 1, preflightCommitment: 'confirmed' },
        );

        console.log(
          ` - Token update index and rate success, tokenIndices - ${tokenIndices}, sig https://explorer.solana.com/tx/${sig.signature}`,
        );
      } catch (e) {
        console.log(
          ` - Token update index and rate error, tokenIndices - ${tokenIndices}, e - ${e}`,
        );
      }
    });
    await new Promise((r) => setTimeout(r, INTERVAL_UPDATE_BANKS * 1000));
  }
}

async function consumeEvents(client: MangoClient, group: Group): Promise<void> {
  console.log('Starting consumeEvents loop');
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const perpMarketIndices = Array.from(
      group.perpMarketsMapByMarketIndex.keys(),
    );
    for (const perpMarketIndex of perpMarketIndices) {
      for (const unused of range(0, 10)) {
        const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
        const eq = await perpMarket.loadEventQueue(client);
        const events = eq.getUnconsumedEvents().slice(0, 10);
        const accounts: Set<PublicKey> = new Set();
        for (const event of events) {
          if (event.eventType === PerpEventQueue.FILL_EVENT_TYPE) {
            accounts.add((event as FillEvent).maker);
            accounts.add((event as FillEvent).taker);
          } else if (event.eventType === PerpEventQueue.OUT_EVENT_TYPE) {
            accounts.add((event as OutEvent).owner);
          } else if (event.eventType === PerpEventQueue.LIQUIDATE_EVENT_TYPE) {
            // pass
          }
        }

        try {
          const sig = await sendTransaction(
            client.program.provider as AnchorProvider,
            [
              await client.perpConsumeEventsIx(
                group,
                perpMarketIndex,
                Array.from(accounts),
                10,
              ),
            ],
            group.addressLookupTablesList,
            { prioritizationFee: 1 },
          );

          console.log(
            ` - Consume events success, perpMarketIndex - ${perpMarketIndex}, sig https://explorer.solana.com/tx/${sig.signature}`,
          );
        } catch (e) {
          console.log(
            ` - Consume events error, perpMarketIndex - ${perpMarketIndex}, e - ${e}`,
          );
        }
      }
    }
    await new Promise((r) => setTimeout(r, INTERVAL_CONSUME_EVENTS * 1000));
  }
}

async function updateFunding(client: MangoClient, group: Group): Promise<void> {
  console.log('Starting updateFunding loop');
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const perpMarketIndices = Array.from(
      group.perpMarketsMapByMarketIndex.keys(),
    );
    for (const perpMarketIndex of perpMarketIndices) {
      try {
        const status = await sendTransaction(
          client.program.provider as AnchorProvider,
          [
            await client.perpUpdateFundingIx(
              group,
              group.getPerpMarketByMarketIndex(perpMarketIndex),
            ),
          ],
          group.addressLookupTablesList,
          { prioritizationFee: 1 },
        );

        console.log(
          ` - Update funding success, perpMarketIndex - ${perpMarketIndex}, sig https://explorer.solana.com/tx/${status.signature}`,
        );
      } catch (e) {
        console.log(
          ` - Update funding error, perpMarketIndex - ${perpMarketIndex}, e - ${e}`,
        );
      }
    }

    await new Promise((r) => setTimeout(r, INTERVAL_UPDATE_FUNDING * 1000));
  }
}

async function checkNewListingsAndAbort(
  client: MangoClient,
  group: Group,
): Promise<void> {
  console.log('Starting checkNewListingsAndAbort loop');
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const freshlyFetchedGroup = await client.getGroup(group.publicKey);
    if (
      freshlyFetchedGroup.banksMapByTokenIndex.size !=
        group.banksMapByTokenIndex.size ||
      freshlyFetchedGroup.perpMarketsMapByMarketIndex.size !=
        group.perpMarketsMapByMarketIndex.size
    ) {
      process.exit();
    }
    await new Promise((r) =>
      setTimeout(r, INTERVAL_CHECK_NEW_LISTINGS_AND_ABORT * 1000),
    );
  }
}

async function keeper(): Promise<void> {
  // Load client
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
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  const mangoAccount = await client.getMangoAccount(
    new PublicKey(MANGO_ACCOUNT_PK),
  );
  const group = await client.getGroup(mangoAccount.group);
  await group.reloadAll(client);

  updateBanks(client, group);
  consumeEvents(client, group);
  updateFunding(client, group);
  checkNewListingsAndAbort(client, group);
}

keeper();
