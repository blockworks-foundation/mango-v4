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
import { Builder } from '../src/builder';
import { MangoClient } from '../src/client';
import { NullTokenEditParams } from '../src/clientIxParamBuilder';
import { MANGO_V4_MAIN_GROUP as MANGO_V4_PRIMARY_GROUP } from '../src/constants';
import { getEquityForMangoAccounts } from '../src/risk';
import { buildFetch } from '../src/utils';
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
  VSR_DELEGATE_KEYPAIR,
  VSR_DELEGATE_FROM_PK,
  DRY_RUN,
} = process.env;

const getApiTokenName = (bankName: string) => {
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

async function updateTokenParams(): Promise<void> {
  const [client, wallet] = await Promise.all([buildClient(), setupWallet()]);
  const vsrClient = await setupVsr(client.connection, wallet);

  const group = await client.getGroup(MANGO_V4_PRIMARY_GROUP);

  const instructions: TransactionInstruction[] = [];

  let mangoAccounts = await client.getAllMangoAccounts(group, true);
  let liqors: PublicKey[];
  liqors = (
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

  const midPriceImpacts = getMidPriceImpacts(group.pis);

  Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    .filter(
      (bank) =>
        // bank.name.includes('bSOL') ||
        // bank.name.includes('JitoSOL') ||
        bank.name.includes('MSOL'),
      // ||
      // bank.name.includes('SOL') ||
      // bank.name.includes('USDT'),
    )
    .forEach(async (bank) => {
      const tokenToPriceImpact = midPriceImpacts
        .filter((x) => x.avg_price_impact_percent < 1)
        .reduce(
          (acc: { [key: string]: MidPriceImpact }, val: MidPriceImpact) => {
            if (
              !acc[val.symbol] ||
              val.target_amount > acc[val.symbol].target_amount
            ) {
              acc[val.symbol] = val;
            }
            return acc;
          },
          {},
        );
      const priceImpact = tokenToPriceImpact[getApiTokenName(bank.name)];
      const newSscaleStartQuote = Math.min(
        // 50 * ttlLiqorEquity,
        4 * priceImpact.target_amount,
        4 * priceImpact.target_amount,
      );
      console.log(`${bank.name} ${newSscaleStartQuote}`);

      const params = Builder(NullTokenEditParams)
        // .borrowWeightScaleStartQuote(priceImpact)
        // .depositWeightScaleStartQuote(priceImpact)
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
          params.flashLoanSwapFeeRate,
          params.interestCurveScaling,
          params.interestTargetUtilization,
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

      instructions.push(ix);
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
      PROPOSAL_TITLE ? PROPOSAL_TITLE : 'Update risk parameters for tokens',
      '',
      Object.values(proposals).length,
      instructions,
      vsrClient!,
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
