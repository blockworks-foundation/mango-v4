import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import uniqWith from 'lodash/uniqWith';
import { TokenIndex } from '../src/accounts/bank';
import { MangoAccount } from '../src/accounts/mangoAccount';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';

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
  const client = await MangoClient.connect(
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
  const serum3Market = Array.from(
    group.serum3MarketsMapByMarketIndex.values(),
  ).filter((m) => m.baseTokenIndex == TOKEN_INDEX)[0];

  const mangoAccountsWithTp = (await client.getAllMangoAccounts(group)).filter(
    (a) => a.getToken(forceWithdrawBank.tokenIndex)?.isActive() ?? false,
  );
  const mangoAccountsWithInUseCount = (
    await client.getAllMangoAccounts(group)
  ).filter((a) => a.getTokenInUseCount(forceWithdrawBank) > 0);

  const mangoAccounts: MangoAccount[] = uniqWith(
    [...mangoAccountsWithTp, ...mangoAccountsWithInUseCount],
    function (a, b) {
      return a.publicKey.equals(b.publicKey);
    },
  );

  console.log(
    `Found ${mangoAccounts.length} mango accounts with in use count > 0 or tp`,
  );

  for (const mangoAccount of mangoAccounts) {
    console.log(
      `${mangoAccount.getTokenBalanceUi(forceWithdrawBank)} for ${
        mangoAccount.publicKey
      }`,
    );

    try {
      const sig = await client.serum3LiqForceCancelOrders(
        group,
        mangoAccount,
        serum3Market.serumMarketExternal,
      );
      console.log(
        ` serum3LiqForceCancelOrders for ${mangoAccount.publicKey}, owner ${
          mangoAccount.owner
        }, sig https://explorer.solana.com/tx/${sig.signature}?cluster=${
          CLUSTER == 'devnet' ? 'devnet' : ''
        }`,
      );
    } catch (error) {
      console.log(error);
    }

    await client
      .tokenForceWithdraw(group, mangoAccount, TOKEN_INDEX)
      .then((sig) => {
        console.log(
          ` tokenForceWithdraw for ${mangoAccount.publicKey}, owner ${
            mangoAccount.owner
          }, sig https://explorer.solana.com/tx/${sig.signature}?cluster=${
            CLUSTER == 'devnet' ? 'devnet' : ''
          }`,
        );
      });
  }

  await group.reloadAll(client);
  console.log(forceWithdrawBank.uiDeposits());
}

forceWithdrawTokens();
