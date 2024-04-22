import {
  LISTING_PRESETS,
  MidPriceImpact,
  getMidPriceImpacts,
} from '@blockworks-foundation/mango-v4-settings/lib/helpers/listingTools';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { BN } from '@project-serum/anchor';
import {
  getAllProposals,
  getTokenOwnerRecord,
  getTokenOwnerRecordAddress,
} from '@solana/spl-governance';
import { Builder } from '../src/builder';

import {
  AccountMeta,
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import fs from 'fs';
import { Bank } from '../src/accounts/bank';
import { Group } from '../src/accounts/group';
import { MangoAccount } from '../src/accounts/mangoAccount';
import { MangoClient } from '../src/client';
import { NullTokenEditParams } from '../src/clientIxParamBuilder';
import { MANGO_V4_MAIN_GROUP as MANGO_V4_PRIMARY_GROUP } from '../src/constants';
import {
  LiqorPriceImpact,
  buildGroupGrid,
  getEquityForMangoAccounts,
} from '../src/risk';
import {
  buildFetch,
  toNativeI80F48ForQuote,
  toUiDecimalsForQuote,
} from '../src/utils';
import {
  MANGO_DAO_WALLET_GOVERNANCE,
  MANGO_GOVERNANCE_PROGRAM,
  MANGO_MINT,
  MANGO_REALM_PK,
} from './governanceInstructions/constants';
import { createProposal } from './governanceInstructions/createProposal';
import {
  DEFAULT_VSR_ID,
  VsrClient,
} from './governanceInstructions/voteStakeRegistryClient';

const {
  MB_CLUSTER_URL,
  PROPOSAL_TITLE,
  PROPOSAL_LINK,
  VSR_DELEGATE_KEYPAIR,
  VSR_DELEGATE_FROM_PK,
  DRY_RUN,
} = process.env;

const getApiTokenName = (bankName: string): string => {
  if (bankName === 'ETH (Portal)') {
    return 'ETH';
  }
  return bankName;
};

async function buildClient(): Promise<MangoClient> {
  return await MangoClient.connectDefault(MB_CLUSTER_URL!);
}

async function setupWallet(): Promise<Wallet> {
  const clientKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(VSR_DELEGATE_KEYPAIR!, 'utf-8'))),
  );
  const clientWallet = new Wallet(clientKeypair);
  return clientWallet;
}

async function setupVsr(
  connection: Connection,
  clientWallet: Wallet,
): Promise<VsrClient> {
  const options = AnchorProvider.defaultOptions();
  const provider = new AnchorProvider(connection, clientWallet, options);
  const vsrClient = await VsrClient.connect(provider, DEFAULT_VSR_ID);
  return vsrClient;
}

async function getTotalLiqorEquity(
  client: MangoClient,
  group: Group,
  mangoAccounts: MangoAccount[],
): Promise<number> {
  const liqors = (
    await (
      await (
        await buildFetch()
      )(
        `https://api.mngo.cloud/data/v4/stats/liqors-over_period?over_period=1MONTH`,
        {
          mode: 'cors',
          headers: {
            'Content-Type': 'application/json',
            'Access-Control-Allow-Origin': '*',
          },
        },
      )
    ).json()
  ).map((data) => new PublicKey(data['liqor']));
  const ttlLiqorEquity = (
    await getEquityForMangoAccounts(client, group, liqors, mangoAccounts)
  ).reduce((partialSum, ae) => partialSum + ae.Equity.val, 0);
  return ttlLiqorEquity;
}

function getPriceImpactForBank(
  midPriceImpacts: MidPriceImpact[],
  bank: Bank,
  priceImpactPercent = 1,
): MidPriceImpact {
  const tokenToPriceImpact = midPriceImpacts
    .filter((x) => x.avg_price_impact_percent < priceImpactPercent)
    .reduce((acc: { [key: string]: MidPriceImpact }, val: MidPriceImpact) => {
      if (
        !acc[val.symbol] ||
        val.target_amount > acc[val.symbol].target_amount
      ) {
        acc[val.symbol] = val;
      }
      return acc;
    }, {});
  const priceImpact = tokenToPriceImpact[getApiTokenName(bank.name)];
  return priceImpact;
}

async function updateTokenParams(): Promise<void> {
  const [client, wallet] = await Promise.all([buildClient(), setupWallet()]);
  const vsrClient = await setupVsr(client.connection, wallet);

  const group = await client.getGroup(MANGO_V4_PRIMARY_GROUP);

  const instructions: TransactionInstruction[] = [];

  const allMangoAccounts = await client.getAllMangoAccounts(group, true);

  const stepSize = 1;

  const ttlLiqorEquityUi = await getTotalLiqorEquity(
    client,
    group,
    allMangoAccounts,
  );

  const midPriceImpacts = getMidPriceImpacts(group.pis);

  const pisForLiqor: LiqorPriceImpact[][] = [];
  // eslint-disable-next-line no-constant-condition
  if (false) {
    const pisForLiqor: LiqorPriceImpact[][] = await buildGroupGrid(
      group,
      allMangoAccounts,
      stepSize,
    );
  }

  // eslint-disable-next-line no-constant-condition
  // if (false) {
  //   // Deposit limits header
  //   console.log(
  //     `${'name'.padStart(20)} ${'maxLiqBatchUi'.padStart(
  //       15,
  //     )} ${'maxLiqBatchUi'.padStart(15)} ${'sellImpact'.padStart(
  //       12,
  //     )}$ ${'pi %'.padStart(12)}% ${'aNDUi'.padStart(
  //       20,
  //     )}${'aNDQuoteUi'.padStart(20)} ${'uiDeposits'.padStart(
  //       12,
  //     )} ${'uiDeposits'.padStart(12)} ${'depositLimitsUi'.padStart(12)}`,
  //   );
  // }

  Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    .sort((a, b) => a.name.localeCompare(b.name))
    .forEach(async (bank) => {
      const builder = Builder(NullTokenEditParams);
      let change = false;

      const tier = Object.values(LISTING_PRESETS).find((x) =>
        x.initLiabWeight.toFixed(1) === '1.8'
          ? x.initLiabWeight.toFixed(1) ===
              bank?.initLiabWeight.toNumber().toFixed(1) &&
            x.reduceOnly === bank.reduceOnly
          : x.initLiabWeight.toFixed(1) ===
            bank?.initLiabWeight.toNumber().toFixed(1),
      );

      if (!tier) {
        console.log(`Cant estimate tier for ${bank.name}!`);
        return;
      }

      // eslint-disable-next-line no-constant-condition
      // if (true) {
      //   if (
      //     bank.uiBorrows() == 0 &&
      //     bank.reduceOnly == 2 &&
      //     bank.initAssetWeight.toNumber() == 0 &&
      //     bank.maintAssetWeight.toNumber() == 0
      //   ) {
      //     builder.disableAssetLiquidation(true);
      //     builder.oracleConfig({
      //       confFilter: 1000,
      //       maxStalenessSlots: -1,
      //     });
      //     change = true;
      //   }
      // }

      // // eslint-disable-next-line no-constant-condition
      // if (true) {
      //   if (bank.uiBorrows() == 0 && bank.reduceOnly == 1) {
      //     builder.disableAssetLiquidation(true);
      //     builder.forceWithdraw(true);
      //     change = true;
      //   }
      // }

      // // eslint-disable-next-line no-constant-condition
      // if (true) {
      //   if (!tier) {
      //     console.log(`${bank.name}, no tier found`);
      //   } else if (tier.preset_name != 'C') {
      //     if (tier.preset_name.includes('A')) {
      //       builder.liquidationFee(bank.liquidationFee.toNumber() * 0.2);
      //       builder.platformLiquidationFee(
      //         bank.liquidationFee.toNumber() * 0.8,
      //       );
      //     } else if (tier.preset_name.includes('B')) {
      //       builder.liquidationFee(bank.liquidationFee.toNumber() * 0.4);
      //       builder.platformLiquidationFee(
      //         bank.liquidationFee.toNumber() * 0.6,
      //       );
      //     }
      //     change = true;
      //   }
      // }

      // eslint-disable-next-line no-constant-condition
      // if (true) {
      //   if (!tier) {
      //     console.log(`${bank.name}, no tier found`);
      //   } else {
      //     console.log(
      //       `${bank.name.padStart(10)}, ${bank.loanFeeRate
      //         .mul(I80F48.fromNumber(100))
      //         .toFixed(2)}, ${bank.loanOriginationFeeRate
      //         .mul(I80F48.fromNumber(100))
      //         .toFixed(2)}, ${tier?.preset_name.padStart(5)}, ${(
      //         tier.loanFeeRate * 100
      //       ).toFixed(2)}, ${(tier!.loanOriginationFeeRate * 100).toFixed(2)}`,
      //     );

      //     builder.loanFeeRate(tier!.loanFeeRate);
      //     builder.loanOriginationFeeRate(tier!.loanOriginationFeeRate);
      //     builder.flashLoanSwapFeeRate(tier!.loanOriginationFeeRate);
      //     change = true;
      //   }
      // }

      // formulas are sourced from here
      // https://www.notion.so/mango-markets/Mango-v4-Risk-parameter-recommendations-d309cdf5faac4aeea7560356e68532ab

      // const priceImpact = getPriceImpactForBank(midPriceImpacts, bank);
      // const scaleStartQuoteUi = Math.min(
      //   50 * ttlLiqorEquityUi,
      //   4 * priceImpact.target_amount,
      // );

      // eslint-disable-next-line no-constant-condition
      if (!bank.areBorrowsReduceOnly()) {
        // Net borrow limits
        let netBorrowLimitPerWindowQuote = Math.max(
          toUiDecimalsForQuote(tier!.netBorrowLimitPerWindowQuote),
          Math.min(bank.uiDeposits() * bank.uiPrice, 300_000) / 3 +
            Math.max(0, bank.uiDeposits() * bank.uiPrice - 300_000) / 5,
        );
        netBorrowLimitPerWindowQuote =
          Math.round(netBorrowLimitPerWindowQuote / 10_000) * 10_000;
        builder.netBorrowLimitPerWindowQuote(
          toNativeI80F48ForQuote(netBorrowLimitPerWindowQuote).toNumber(),
        );
        change = true;
        if (
          netBorrowLimitPerWindowQuote !=
          toUiDecimalsForQuote(bank.netBorrowLimitPerWindowQuote)
        ) {
          console.log(
            `${bank.name}, ${bank.uiDeposits() * bank.uiPrice}$ (${
              Math.min(bank.uiDeposits() * bank.uiPrice, 300_000) / 3
            }, ${
              Math.max(0, bank.uiDeposits() * bank.uiPrice - 300_000) / 5
            }), ${
              tier?.netBorrowLimitPerWindowQuote
            } , new - ${netBorrowLimitPerWindowQuote}, old - ${toUiDecimalsForQuote(
              bank.netBorrowLimitPerWindowQuote,
            )}`,
          );
        }
      }

      // Deposit limits
      // eslint-disable-next-line no-constant-condition
      // if (false) {
      //   if (bank.maintAssetWeight.toNumber() > 0) {
      //     {
      //       // Find asset's largest batch in $ we would need to liquidate, batches are extreme points of a range of price drop,
      //       // range is constrained by leverage provided
      //       // i.e. how much volatility we expect
      //       const r = findLargestAssetBatchUi(
      //         pisForLiqor,
      //         bank.name,
      //         Math.round(bank.maintAssetWeight.toNumber() * 100),
      //         100 - Math.round(bank.maintAssetWeight.toNumber() * 100),
      //         stepSize,
      //       );

      //       const maxLiqBatchQuoteUi = r[0];
      //       const maxLiqBatchUi = r[1];

      //       const sellImpact = getPriceImpactForBank(
      //         midPriceImpacts,
      //         bank,
      //         (bank.liquidationFee.toNumber() * 100) / 2,
      //       );

      //       // Deposit limit = sell impact - largest batch
      //       const allowedNewDepositsQuoteUi =
      //         sellImpact.target_amount - maxLiqBatchQuoteUi;
      //       const allowedNewDepositsUi =
      //         sellImpact.target_amount / bank.uiPrice -
      //         maxLiqBatchQuoteUi / bank.uiPrice;

      //       const depositLimitUi = bank.uiDeposits() + allowedNewDepositsUi;

      //       // LOG
      //       // console.log(
      //       //   `${bank.name.padStart(20)} ${maxLiqBatchUi
      //       //     .toLocaleString()
      //       //     .padStart(15)} ${maxLiqBatchQuoteUi
      //       //     .toLocaleString()
      //       //     .padStart(15)}$ ${sellImpact.target_amount
      //       //     .toLocaleString()
      //       //     .padStart(12)}$ ${sellImpact.avg_price_impact_percent
      //       //     .toLocaleString()
      //       //     .padStart(12)}% ${allowedNewDepositsUi
      //       //     .toLocaleString()
      //       //     .padStart(20)}${allowedNewDepositsQuoteUi
      //       //     .toLocaleString()
      //       //     .padStart(20)}$ ${bank
      //       //     .uiDeposits()
      //       //     .toLocaleString()
      //       //     .padStart(12)} ${(bank.uiDeposits() * bank.uiPrice)
      //       //     .toLocaleString()
      //       //     .padStart(12)}$ ${depositLimitUi
      //       //     .toLocaleString()
      //       //     .padStart(12)}`,
      //       // );

      //       builder.depositLimit(toNative(depositLimitUi, bank.mintDecimals));
      //       change = true;
      //     }
      //   }
      // }

      const params = builder.build();

      const ix = await client.program.methods
        .tokenEdit(
          params.oracle,
          params.oracleConfig,
          params.groupInsuranceFund,
          params.interestRateParams,
          params.loanFeeRate,
          params.loanOriginationFeeRate,
          params.maintAssetWeight,
          params.initAssetWeight,
          params.maintLiabWeight,
          params.initLiabWeight,
          params.liquidationFee,
          params.stablePriceDelayIntervalSeconds,
          params.stablePriceDelayGrowthLimit,
          params.stablePriceGrowthLimit,
          params.minVaultToDepositsRatio,
          params.netBorrowLimitPerWindowQuote !== null
            ? new BN(params.netBorrowLimitPerWindowQuote)
            : null,
          params.netBorrowLimitWindowSizeTs !== null
            ? new BN(params.netBorrowLimitWindowSizeTs)
            : null,
          params.borrowWeightScaleStartQuote,
          params.depositWeightScaleStartQuote,
          params.resetStablePrice ?? false,
          params.resetNetBorrowLimit ?? false,
          params.reduceOnly,
          params.name,
          params.forceClose,
          params.tokenConditionalSwapTakerFeeRate,
          params.tokenConditionalSwapMakerFeeRate,
          params.flashLoanSwapFeeRate,
          params.interestCurveScaling,
          params.interestTargetUtilization,
          params.maintWeightShiftStart,
          params.maintWeightShiftEnd,
          params.maintWeightShiftAssetTarget,
          params.maintWeightShiftLiabTarget,
          params.maintWeightShiftAbort ?? false,
          false, // setFallbackOracle, unused
          params.depositLimit,
          params.zeroUtilRate,
          params.platformLiquidationFee,
          params.disableAssetLiquidation,
          params.collateralFeePerDay,
          params.forceWithdraw,
        )
        .accounts({
          group: group.publicKey,
          oracle: bank.oracle,
          admin: group.admin,
          mintInfo: group.mintInfosMapByTokenIndex.get(bank.tokenIndex)
            ?.publicKey,
          fallbackOracle: PublicKey.default,
        })
        .remainingAccounts([
          {
            pubkey: bank.publicKey,
            isWritable: true,
            isSigner: false,
          } as AccountMeta,
        ])
        .instruction();

      const tx = new Transaction({ feePayer: wallet.publicKey }).add(ix);
      const simulated = await client.connection.simulateTransaction(tx);

      if (simulated.value.err) {
        console.log('error', simulated.value.logs);
        throw simulated.value.logs;
      }

      if (change) {
        instructions.push(ix);
      }
    });

  const tokenOwnerRecordPk = await getTokenOwnerRecordAddress(
    MANGO_GOVERNANCE_PROGRAM,
    MANGO_REALM_PK,
    MANGO_MINT,
    new PublicKey(VSR_DELEGATE_FROM_PK!),
  );

  const [tokenOwnerRecord, proposals] = await Promise.all([
    getTokenOwnerRecord(client.connection, tokenOwnerRecordPk),
    getAllProposals(
      client.connection,
      MANGO_GOVERNANCE_PROGRAM,
      MANGO_REALM_PK,
    ),
  ]);

  const walletSigner = wallet as never;

  if (!DRY_RUN) {
    const proposalAddress = await createProposal(
      client.connection,
      walletSigner,
      MANGO_DAO_WALLET_GOVERNANCE,
      tokenOwnerRecord,
      PROPOSAL_TITLE
        ? PROPOSAL_TITLE
        : 'Update net borrow limits for tokens in mango-v4',
      PROPOSAL_LINK ?? '',
      Object.values(proposals).length,
      instructions,
      vsrClient!,
      false,
    );
    console.log(proposalAddress.toBase58());
  }
}

async function main(): Promise<void> {
  try {
    await updateTokenParams();
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
