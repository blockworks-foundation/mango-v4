import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import {
  Bank,
  I80F48,
  MANGO_V4_ID,
  MANGO_V4_MAIN_GROUP,
  MangoClient,
  ONE_I80F48,
  sleep,
  toUiDecimals,
} from '../src';
import { HealthCache } from '../src/accounts/healthCache';
import BN from 'bn.js';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK;
const DRY_RUN = parseEnvBoolean(process.env.DRY_RUN); // Set to true if you don't want to execute txs
const DEPOSIT = parseEnvBoolean(process.env.DEPOSIT); // set to true if you just want to deposit into accounts

async function forceCloseAllBorrows(): Promise<void> {
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

  const liqor = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK!));
  const group = await client.getGroup(MANGO_V4_MAIN_GROUP);
  const allMangoAccounts = await client.getAllMangoAccounts(group, true);

  // Get all banks with borrows and sort descending order
  const liabBanks: Bank[] = [];
  for (const [, v] of group.banksMapByName) {
    if (v.length !== 1) {
      throw new Error('!');
    }
    const bank = v[0];
    if (bank.uiBorrows() > 0 && bank.name !== 'USDC') {
      liabBanks.push(bank);
    }
  }
  liabBanks.sort(
    (a, b) => b.uiBorrows() * b.uiPrice - a.uiBorrows() * a.uiPrice,
  );

  // Assume that all tokens are in forceClose
  for (const liabBank of liabBanks) {
    // Find all mango accounts that have borrows in this liabBank then sort largest borrow to smallest borrow
    const accountsWithBorrows = allMangoAccounts
      .filter((a) => {
        const hc = HealthCache.fromMangoAccount(group, a);
        const i = hc.findTokenInfoIndex(liabBank.tokenIndex);
        return i !== -1 && hc.tokenInfos[i].balanceSpot.isNeg();
      })
      .sort((a, b) => {
        const hca = HealthCache.fromMangoAccount(group, a);
        const hcb = HealthCache.fromMangoAccount(group, b);
        return hca.tokenInfos[
          hca.findTokenInfoIndex(liabBank.tokenIndex)
        ].balanceSpot
          .sub(
            hcb.tokenInfos[hcb.findTokenInfoIndex(liabBank.tokenIndex)]
              .balanceSpot,
          )
          .toNumber();
      });

    for (const liqee of accountsWithBorrows) {
      console.log(
        `Liquidating ${liqee.publicKey} - ${liqee.getTokenBalanceUi(liabBank)} ${liabBank.name}`,
      );
      if (!DEPOSIT) {
        // Find all assets of liqee
        const assetBanks: Bank[] = [];
        const hc = HealthCache.fromMangoAccount(group, liqee);
        for (const tokenInfo of hc.tokenInfos) {
          if (tokenInfo.balanceSpot.isPos()) {
            const assetBank = group.getFirstBankByTokenIndex(
              tokenInfo.tokenIndex,
            );
            assetBanks.push(assetBank);
          }
        }
        // Sort assets by liq fee
        assetBanks.sort((a, b) =>
          a.liquidationFee
            .add(a.platformLiquidationFee)
            .sub(b.liquidationFee)
            .sub(b.platformLiquidationFee)
            .toNumber(),
        );

        // Now iterate over each asset and make sure you liquidate enough asset such that it does not go negative
        const liabInfo = hc.tokenInfos.find(
          (x) => x.tokenIndex === liabBank.tokenIndex,
        )!;
        for (const assetBank of assetBanks) {
          const assetInfo = hc.tokenInfos.find(
            (x) => x.tokenIndex === assetBank.tokenIndex,
          )!;

          const epsilon = I80F48.fromNumber(10000);
          const assetValueNative = liqee
            .getTokenDeposits(assetBank)
            .mul(assetInfo.assetWeightedPrice(undefined))
            .sub(epsilon);

          if (!assetValueNative.isPos()) continue;

          // fee_factor_total = (1 + liab_liq_fee + liab_pliq_fee) * (1 + asset_liq_fee + asset_pliq_fee)
          const feeFactorTotal = ONE_I80F48()
            .add(liabBank.liquidationFee)
            .add(liabBank.platformLiquidationFee)
            .mul(
              ONE_I80F48()
                .add(assetBank.liquidationFee)
                .add(assetBank.platformLiquidationFee),
            );

          const maxLiabTransferNative = assetValueNative
            .div(feeFactorTotal)
            .div(liabInfo.liabWeightedPrice(undefined));
          const maxLiabTransfer = toUiDecimals(
            maxLiabTransferNative,
            liabBank.mintDecimals,
          );

          console.log(
            `liab - ${liabBank.name} - ${toUiDecimals(liabInfo.balanceSpot, liabBank.mintDecimals)} asset: ${assetBank.name} - ${toUiDecimals(assetInfo.balanceSpot, assetBank.mintDecimals)} maxLiabTransfer - ${maxLiabTransfer}`,
          );
          if (liabInfo.balanceSpot.isZero()) continue;
          if (!DRY_RUN) {
            try {
              const sig = await client.tokenForceCloseBorrowsWithToken(
                group,
                liqor,
                liqee,
                assetBank.tokenIndex,
                liabBank.tokenIndex,
                maxLiabTransfer,
              );
              console.log(` - sig ${sig.signature}`);

              // Reload account and see if there are still borrows left
              await sleep(1000);
              await Promise.all([liqee.reload(client), liqor.reload(client)]);
            } catch (e) {
              console.log(e);
              continue;
            }
          }
          // TODO handle dust borrows case
          console.log(
            `liqee ${liqee.publicKey} liab: ${liqee.getTokenBorrowsUi(liabBank)}`,
          );
          if (liqee.getTokenBorrowsUi(liabBank) === 0) break;
        }
      } else {
        // Since total borrows are so low, just offset borrows with token deposits from liqor
        try {
          const liabNative = liqee.getTokenBorrows(liabBank);
          console.log(
            `depositing ${new BN(liabNative.ceil().toString())} or ${toUiDecimals(liabNative.ceil(), liabBank.mintDecimals)}`,
          );
          if (!DRY_RUN) {
            const sig = await client.tokenDepositNative(
              group,
              liqee,
              liabBank.mint,
              new BN(liabNative.ceil().toString()),
              true,
              true,
            );
            console.log(`sig - ${sig.signature}`);
            liqee.reload(client);
            await sleep(100);
          }
        } catch (e) {
          console.log(e);
        }
      }
    }
  }

  // Print out how much of each liab token the liqor needs to keep in his account
  if (DRY_RUN) {
    console.log(`Necessary assets for liqor`);
    console.log(`---------------------------------`);
    for (const liabBank of liabBanks) {
      console.log(
        `${liabBank.name} - ${liabBank.uiBorrows()} - $${liabBank.uiBorrows() * liabBank.uiPrice}`,
      );
    }
  }
}

function parseEnvBoolean(value: string | undefined): boolean {
  if (!value) return false;

  const truthyValues = ['true', '1', 'yes', 'y'];
  return truthyValues.includes(value.toLowerCase());
}

forceCloseAllBorrows();
