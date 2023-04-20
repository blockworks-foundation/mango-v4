import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../src/accounts/bank';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import {
  fetchJupiterTransaction,
  fetchRoutes,
  prepareMangoRouterInstructions,
} from './router';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK;
const TOKEN_INDEX = Number(process.env.TOKEN_INDEX) as TokenIndex;

async function forceCloseTokenBorrows(): Promise<void> {
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

  const liqor = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK!));
  const group = await client.getGroup(liqor.group);
  const forceCloseTokenBank = group.getFirstBankByTokenIndex(TOKEN_INDEX);
  const usdcBank = group.getFirstBankByTokenIndex(0 as TokenIndex);
  const mangoAccountsWithBorrows = (
    await client.getAllMangoAccounts(group)
  ).filter((a) => a.getTokenBalanceUi(forceCloseTokenBank) < 0);

  for (const liqee of mangoAccountsWithBorrows) {
    const amount =
      liqee.getTokenBorrowsUi(forceCloseTokenBank) *
      forceCloseTokenBank.uiPrice *
      (1 + forceCloseTokenBank.liquidationFee.toNumber());

    const { bestRoute } = await fetchRoutes(
      usdcBank.mint,
      forceCloseTokenBank.mint,
      amount.toString(),
    );
    if (!bestRoute) {
      continue;
    }
    const [ixs, alts] =
      bestRoute.routerName === 'Mango'
        ? await prepareMangoRouterInstructions(
            bestRoute,
            usdcBank.mint,
            forceCloseTokenBank.mint,
            user.publicKey,
          )
        : await fetchJupiterTransaction(
            this.client.connection,
            bestRoute,
            user.publicKey,
            0,
            usdcBank.mint,
            forceCloseTokenBank.mint,
          );
    await this.client.marginTrade({
      group: this.group,
      mangoAccount: this.mangoAccount,
      inputMintPk: usdcBank.mint,
      amountIn: amount,
      outputMintPk: usdcBank.mint,
      userDefinedInstructions: ixs,
      userDefinedAlts: alts,
      flashLoanType: { swap: {} },
    });

    await client.tokenForceCloseBorrowsWithToken(
      group,
      liqor,
      liqee,
      usdcBank.tokenIndex,
      forceCloseTokenBank.tokenIndex,
    );
  }
}

forceCloseTokenBorrows();
