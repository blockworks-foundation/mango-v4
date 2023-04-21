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
} from '../src/router';
import { toNative, toUiDecimals } from '../src/utils';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK;
const TOKEN_INDEX = Number(process.env.TOKEN_INDEX) as TokenIndex;
const MAX_LIAB_TRANSFER = Number(process.env.MAX_LIAB_TRANSFER);

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

  let liqor = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK!));
  const group = await client.getGroup(liqor.group);
  const forceCloseTokenBank = group.getFirstBankByTokenIndex(TOKEN_INDEX);
  if (forceCloseTokenBank.reduceOnly != 2) {
    throw new Error(
      `Unexpected reduce only state ${forceCloseTokenBank.reduceOnly}`,
    );
  }
  if (!forceCloseTokenBank.forceClose) {
    throw new Error(
      `Unexpected force close state ${forceCloseTokenBank.forceClose}`,
    );
  }

  const usdcBank = group.getFirstBankByTokenIndex(0 as TokenIndex);
  // Get all mango accounts with borrows for given token
  const mangoAccountsWithBorrows = (
    await client.getAllMangoAccounts(group)
  ).filter((a) => a.getTokenBalanceUi(forceCloseTokenBank) < 0);

  console.log(`${liqor.toString(group, true)}`);

  for (const liqee of mangoAccountsWithBorrows) {
    liqor = await liqor.reload(client);
    // Liqor can only liquidate borrow using deposits, since borrows are in reduce only
    // Swap usdc worth token borrow (sub existing position), account for slippage using liquidation fee
    // MAX_LIAB_TRANSFER guards against trying to swap to a very large amount
    const amount =
      Math.min(
        liqee.getTokenBorrowsUi(forceCloseTokenBank) -
          liqor.getTokenBalanceUi(forceCloseTokenBank),
        MAX_LIAB_TRANSFER,
      ) *
      forceCloseTokenBank.uiPrice *
      (1 + forceCloseTokenBank.liquidationFee.toNumber());

    console.log(
      `liqor balance ${liqor.getTokenBalanceUi(
        forceCloseTokenBank,
      )}, liqee balance ${liqee.getTokenBalanceUi(
        forceCloseTokenBank,
      )}, liqor will swap further amount of $${toUiDecimals(
        amount,
        usdcBank.mintDecimals,
      )} to ${forceCloseTokenBank.name}`,
    );

    const amountBn = toNative(
      Math.min(amount, 99999999999), // Jupiter API can't handle amounts larger than 99999999999
      usdcBank.mintDecimals,
    );
    const { bestRoute } = await fetchRoutes(
      usdcBank.mint,
      forceCloseTokenBank.mint,
      amountBn.toString(),
      forceCloseTokenBank.liquidationFee.toNumber() * 100,
      'ExactIn',
      '0',
      liqor.owner,
    );
    if (!bestRoute) {
      await new Promise((r) => setTimeout(r, 500));
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
            client.connection,
            bestRoute,
            user.publicKey,
            0,
            usdcBank.mint,
            forceCloseTokenBank.mint,
          );
    const sig = await client.marginTrade({
      group: group,
      mangoAccount: liqor,
      inputMintPk: usdcBank.mint,
      amountIn: amount,
      outputMintPk: forceCloseTokenBank.mint,
      userDefinedInstructions: ixs,
      userDefinedAlts: alts,
      flashLoanType: { swap: {} },
    });
    console.log(
      ` - marginTrade, sig https://explorer.solana.com/tx/${sig}?cluster=${
        CLUSTER == 'devnet' ? 'devnet' : ''
      }`,
    );

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
