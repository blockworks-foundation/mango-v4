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
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK;
const TOKEN_INDEX = Number(process.env.TOKEN_INDEX) as TokenIndex;

async function tokenDeposit(): Promise<void> {
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
      prioritizationFee: 80000,
    },
  );

  const liqor = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK!));
  const group = await client.getGroup(liqor.group);

  const forceCloseTokenBank = group.getFirstBankByTokenIndex(TOKEN_INDEX);

  const mangoAccountsWithBorrows = (
    await client.getAllMangoAccounts(group)
  ).filter((a) => a.getTokenBalanceUi(forceCloseTokenBank) < 0);

  for (const liqee of mangoAccountsWithBorrows) {
    console.log(
      `liqee ${liqee.publicKey}, ${liqee.getTokenBalanceUi(
        forceCloseTokenBank,
      )}`,
    );

    const sig = await client.tokenDeposit(
      group,
      liqee,
      forceCloseTokenBank.mint,
      -liqee.getTokenBalanceUi(forceCloseTokenBank),
    );

    console.log(
      ` - tokendeposit, sig https://explorer.solana.com/tx/${
        sig.signature
      }?cluster=${CLUSTER == 'devnet' ? 'devnet' : ''}`,
    );
  }
}

tokenDeposit();
