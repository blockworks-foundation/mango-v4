import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../src/accounts/bank';
import { HealthType } from '../src/accounts/mangoAccount';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { ONE_I80F48 } from '../src/numbers/I80F48';

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

  const mangoAccountsWithBorrows = (
    await client.getAllMangoAccounts(group)
  ).filter((a) => a.getTokenBalanceUi(forceCloseTokenBank) < 0);

  if (
    forceCloseTokenBank.uiBorrows() >=
    liqor.getTokenBalanceUi(forceCloseTokenBank)
  ) {
    throw new Error(
      `Ensure that liqor has enough deposits to cover borrows! forceCloseTokenBank.uiBorrows() ${forceCloseTokenBank.uiBorrows()}, liqor.getTokenBalanceUi(forceCloseTokenBank) ${liqor.getTokenBalanceUi(forceCloseTokenBank)}`,
    );
  }

  console.log(`${liqor.toString(group, true)}`);

  for (const liqee of mangoAccountsWithBorrows) {
    liqor = await liqor.reload(client);

    const sortedByContribution = liqee
      .getHealthContributionPerAssetUi(group, HealthType.init)
      .filter((a) => {
        const potentialAssetBank = group.getFirstBankByName(a.asset);

        const feeFactorTotal = ONE_I80F48()
          .add(forceCloseTokenBank.liquidationFee)
          .add(forceCloseTokenBank.platformLiquidationFee)
          .mul(
            ONE_I80F48()
              .add(potentialAssetBank.liquidationFee)
              .add(potentialAssetBank.platformLiquidationFee),
          );

        return (
          potentialAssetBank.reduceOnly != 2 &&
          forceCloseTokenBank.initLiabWeight.gte(
            potentialAssetBank.initLiabWeight.mul(feeFactorTotal),
          )
        );
      })
      .sort((a, b) => {
        return a.contribution - b.contribution;
      });
    const assetBank = group.getFirstBankByName(sortedByContribution[0].asset);

    console.log(
      `${liqee.publicKey.toString()}, balance ${liqee.getTokenBalanceUi(forceCloseTokenBank)}, asset ${assetBank.name}, contribution ${sortedByContribution[0].contribution}`,
    );

    const sig = await client.tokenForceCloseBorrowsWithToken(
      group,
      liqor,
      liqee,
      assetBank.tokenIndex,
      forceCloseTokenBank.tokenIndex,
    );
    console.log(` - sig ${sig.signature}`);
  }
}

forceCloseTokenBorrows();
