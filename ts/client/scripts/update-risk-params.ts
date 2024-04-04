import {
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
import { Builder } from '../src/builder';
import { MangoClient } from '../src/client';
import { NullTokenEditParams } from '../src/clientIxParamBuilder';
import { MANGO_V4_MAIN_GROUP as MANGO_V4_PRIMARY_GROUP } from '../src/constants';
import {
  LiqorPriceImpact,
  buildGroupGrid,
  findLargestAssetBatchUi,
  getEquityForMangoAccounts,
} from '../src/risk';
import {
  buildFetch,
  toNative,
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

  const pisForLiqor: LiqorPriceImpact[][] = await buildGroupGrid(
    group,
    allMangoAccounts,
    stepSize,
  );

  // Deposit limits header
  console.log(
    `${'name'.padStart(20)} ${'maxLiqBatchUi'.padStart(
      15,
    )} ${'maxLiqBatchUi'.padStart(15)} ${'sellImpact'.padStart(
      12,
    )}$ ${'pi %'.padStart(12)}% ${'aNDUi'.padStart(20)}${'aNDQuoteUi'.padStart(
      20,
    )} ${'uiDeposits'.padStart(12)} ${'uiDeposits'.padStart(
      12,
    )} ${'depositLimitsUi'.padStart(12)}`,
  );

  Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    // .filter((bank) => bank.name == 'MSOL')
    .forEach(async (bank) => {
      let change = false;
      try {
        // formulas are sourced from here
        // https://www.notion.so/mango-markets/Mango-v4-Risk-parameter-recommendations-d309cdf5faac4aeea7560356e68532ab

        // const priceImpact = getPriceImpactForBank(midPriceImpacts, bank);
        // const scaleStartQuoteUi = Math.min(
        //   50 * ttlLiqorEquityUi,
        //   4 * priceImpact.target_amount,
        // );

        const builder = Builder(NullTokenEditParams);

        // Net borrow limits
        if (!bank.areBorrowsReduceOnly()) {
          const netBorrowLimitPerWindowQuote = Math.max(
            10_000,
            Math.min(bank.uiDeposits() * bank.uiPrice, 300_000) / 3 +
              Math.max(0, bank.uiDeposits() * bank.uiPrice - 300_000) / 5,
          );
          builder.netBorrowLimitPerWindowQuote(
            toNativeI80F48ForQuote(netBorrowLimitPerWindowQuote).toNumber(),
          );
          change = true;
          if (
            netBorrowLimitPerWindowQuote !=
            toUiDecimalsForQuote(bank.netBorrowLimitPerWindowQuote)
          ) {
            console.log(
              `${
                bank.name
              } new - ${netBorrowLimitPerWindowQuote.toLocaleString()}, old - ${toUiDecimalsForQuote(
                bank.netBorrowLimitPerWindowQuote,
              ).toLocaleString()}`,
            );
          }

          // Deposit limits
          if (bank.maintAssetWeight.toNumber() > 0) {
            {
              // Find asset's largest batch in $ we would need to liquidate, batches are extreme points of a range of price drop,
              // range is constrained by leverage provided
              // i.e. how much volatility we expect
              const r = findLargestAssetBatchUi(
                pisForLiqor,
                bank.name,
                Math.round(bank.maintAssetWeight.toNumber() * 100),
                100 - Math.round(bank.maintAssetWeight.toNumber() * 100),
                stepSize,
              );

              const maxLiqBatchQuoteUi = r[0];
              const maxLiqBatchUi = r[1];

              const sellImpact = getPriceImpactForBank(
                midPriceImpacts,
                bank,
                (bank.liquidationFee.toNumber() * 100) / 2,
              );

              // Deposit limit = sell impact - largest batch
              const allowedNewDepositsQuoteUi =
                sellImpact.target_amount - maxLiqBatchQuoteUi;
              const allowedNewDepositsUi =
                sellImpact.target_amount / bank.uiPrice -
                maxLiqBatchQuoteUi / bank.uiPrice;

              let depositLimitUi = bank.uiDeposits() + allowedNewDepositsUi;

              if (bank.name == 'JitoSOL') {
                depositLimitUi = Math.min(depositLimitUi, 12_000);
              }
              if (bank.name == 'bSOL') {
                depositLimitUi = Math.min(depositLimitUi, 6_000);
              }
              if (bank.name == 'MSOL') {
                depositLimitUi = Math.min(depositLimitUi, 50_000);
              }
              if (bank.name == 'JLP') {
                depositLimitUi = Math.min(depositLimitUi, 300_000);
              }
              if (bank.name == 'RAY') {
                depositLimitUi = Math.min(depositLimitUi, 300_000);
              }
              if (bank.name == 'wBTC (Portal)') {
                depositLimitUi = Math.max(depositLimitUi, 7.5);
              }
              if (bank.name == 'SOL') {
                depositLimitUi = Math.max(depositLimitUi, 50_000);
              }

              console.log(
                `${bank.name.padStart(20)} ${maxLiqBatchUi
                  .toLocaleString()
                  .padStart(15)} ${maxLiqBatchQuoteUi
                  .toLocaleString()
                  .padStart(15)}$ ${sellImpact.target_amount
                  .toLocaleString()
                  .padStart(12)}$ ${sellImpact.avg_price_impact_percent
                  .toLocaleString()
                  .padStart(12)}% ${allowedNewDepositsUi
                  .toLocaleString()
                  .padStart(20)}${allowedNewDepositsQuoteUi
                  .toLocaleString()
                  .padStart(20)}$ ${bank
                  .uiDeposits()
                  .toLocaleString()
                  .padStart(12)} ${(bank.uiDeposits() * bank.uiPrice)
                  .toLocaleString()
                  .padStart(12)}$ ${depositLimitUi
                  .toLocaleString()
                  .padStart(12)}`,
              );

              builder.depositLimit(toNative(depositLimitUi, bank.mintDecimals));
              change = true;
            }
          }

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

          const tx = new Transaction({ feePayer: wallet.publicKey }).add(ix);
          const simulated = await client.connection.simulateTransaction(tx);

          if (simulated.value.err) {
            console.log('error', simulated.value.logs);
            throw simulated.value.logs;
          }

          if (change) {
            instructions.push(ix);
          }
        }
      } catch (error) {
        console.log(`....Skipping ${bank.name}, ${error}`);
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

  console.log(DRY_RUN);

  if (!DRY_RUN) {
    const proposalAddress = await createProposal(
      client.connection,
      walletSigner,
      MANGO_DAO_WALLET_GOVERNANCE,
      tokenOwnerRecord,
      PROPOSAL_TITLE ? PROPOSAL_TITLE : 'Update deposit limits for tokens',
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
