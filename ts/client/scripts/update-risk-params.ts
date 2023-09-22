import { BN } from '@project-serum/anchor';
import { serializeInstructionToBase64 } from '@solana/spl-governance';
import { AccountMeta } from '@solana/web3.js';
import { Builder } from '../src/builder';
import { MangoClient } from '../src/client';
import { NullTokenEditParams } from '../src/clientIxParamBuilder';
import { MANGO_V4_MAIN_GROUP as MANGO_V4_PRIMARY_GROUP } from '../src/constants';
import { computePriceImpactOnJup } from '../src/risk';
import { toNative } from '../src/utils';

const { MB_CLUSTER_URL } = process.env;

async function buildClient(): Promise<MangoClient> {
  return await MangoClient.connectDefault(MB_CLUSTER_URL!);
}

async function updateTokenParams(): Promise<void> {
  const client = await buildClient();

  const group = await client.getGroup(MANGO_V4_PRIMARY_GROUP);

  Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    .forEach(async (bank) => {
      const usdcAmounts = [
        1_000, 5_000, 20_000, 100_000, 250_000, 500_000, 1_000_000, 5_000_000,
      ];

      // Limit borrows to 1/3rd of deposit, rounded to 1000, only update if more than 10% different
      const depositsInUsd = bank.nativeDeposits().mul(bank.price);
      let newNetBorrowLimitPerWindowQuote: number | null =
        depositsInUsd.toNumber() / 3;
      newNetBorrowLimitPerWindowQuote =
        Math.round(newNetBorrowLimitPerWindowQuote / 1_000_000_000) *
        1_000_000_000;
      newNetBorrowLimitPerWindowQuote =
        Math.abs(
          (newNetBorrowLimitPerWindowQuote -
            bank.netBorrowLimitPerWindowQuote.toNumber()) /
            bank.netBorrowLimitPerWindowQuote.toNumber(),
        ) > 0.1
          ? newNetBorrowLimitPerWindowQuote
          : null;

      // Kick in weight scaling as late as possible until liquidation fee remains reasonable
      // Only update if more than 10% different
      const index = usdcAmounts
        .map((usdcAmount) => {
          const piFraction =
            computePriceImpactOnJup(group.pis, usdcAmount, bank.name) / 10_000;
          return bank.liquidationFee.toNumber() / 1.5 > piFraction;
        })
        .lastIndexOf(true);
      let newWeightScaleQuote =
        index > -1 ? new BN(toNative(usdcAmounts[index], 6)).toNumber() : null;
      newWeightScaleQuote =
        newWeightScaleQuote != null &&
        Math.abs(
          (newWeightScaleQuote - bank.depositWeightScaleStartQuote) /
            bank.depositWeightScaleStartQuote,
        ) > 0.1
          ? newWeightScaleQuote
          : null;

      const params = Builder(NullTokenEditParams)
        .netBorrowLimitPerWindowQuote(newNetBorrowLimitPerWindowQuote)
        .borrowWeightScaleStartQuote(newWeightScaleQuote)
        .depositWeightScaleStartQuote(newWeightScaleQuote)
        .build();

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
          params.flashLoanDepositFeeRate,
        )
        .accounts({
          group: group.publicKey,
          oracle: bank.oracle,
          admin: group.admin,
          mintInfo: group.mintInfosMapByTokenIndex.get(bank.tokenIndex)
            ?.publicKey,
        })
        .remainingAccounts([
          {
            pubkey: bank.publicKey,
            isWritable: true,
            isSigner: false,
          } as AccountMeta,
        ])
        .instruction();

      console.log(`Bank ${bank.name} ${serializeInstructionToBase64(ix)}`);
    });
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
