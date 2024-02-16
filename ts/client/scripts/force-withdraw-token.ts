import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../src/accounts/bank';
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
  if (forceWithdrawBank.reduceOnly != 2) {
    throw new Error(
      `Unexpected reduce only state ${forceWithdrawBank.reduceOnly}`,
    );
  }
  if (!forceWithdrawBank.forceWithdraw) {
    throw new Error(
      `Unexpected force withdraw state ${forceWithdrawBank.forceWithdraw}`,
    );
  }

  // Get all mango accounts with deposits for given token
  const mangoAccountsWithDeposits = (
    await client.getAllMangoAccounts(group)
  ).filter((a) => a.getTokenBalanceUi(forceWithdrawBank) > 0);

  for (const mangoAccount of mangoAccountsWithDeposits) {
    const sig = await client.tokenForceWithdraw(
      group,
      mangoAccount,
      TOKEN_INDEX,
    );
    console.log(
      ` tokenForceWithdraw for ${mangoAccount.publicKey}, owner ${
        mangoAccount.owner
      }, sig https://explorer.solana.com/tx/${sig}?cluster=${
        CLUSTER == 'devnet' ? 'devnet' : ''
      }`,
    );
  }
}

forceWithdrawTokens();
