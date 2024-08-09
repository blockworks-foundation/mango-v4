import { LISTING_PRESETS } from '@blockworks-foundation/mango-v4-settings/lib/helpers/listingTools';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  getAllProposals,
  getTokenOwnerRecord,
  getTokenOwnerRecordAddress,
} from '@solana/spl-governance';
import {
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  AggregatorAccount,
  SwitchboardProgram,
} from '@switchboard-xyz/solana.js';
import fs from 'fs';
import { OracleProvider } from '../src';
import { MangoClient } from '../src/client';
import { MANGO_V4_MAIN_GROUP as MANGO_V4_PRIMARY_GROUP } from '../src/constants';
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

async function withdrawFeesToAdmin(): Promise<void> {
  const [client, wallet] = await Promise.all([buildClient(), setupWallet()]);
  const vsrClient = await setupVsr(client.connection, wallet);

  const group = await client.getGroup(MANGO_V4_PRIMARY_GROUP);

  const instructions: TransactionInstruction[] = [];

  const sbProgram = await SwitchboardProgram.load(
    'mainnet-beta',
    client.connection,
  );

  Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    .sort((a, b) => a.name.localeCompare(b.name))
    .forEach(async (bank) => {
      const tier = Object.values(LISTING_PRESETS).find((x) =>
        x.initLiabWeight.toFixed(1) === '1.8'
          ? x.initLiabWeight.toFixed(1) ===
              bank?.initLiabWeight.toNumber().toFixed(1) &&
            x.reduceOnly === bank.reduceOnly
          : x.initLiabWeight.toFixed(1) ===
            bank?.initLiabWeight.toNumber().toFixed(1),
      );

      if (bank.oracleProvider == OracleProvider.Switchboard) {
        const aggregatorAccount = new AggregatorAccount(sbProgram, bank.oracle);
        const data = await aggregatorAccount.loadData();
        console.log(
          `${bank.name} ${tier?.preset_name} ${data.minUpdateDelaySeconds}`,
        );
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
      PROPOSAL_TITLE ? PROPOSAL_TITLE : 'Withdraw token fees in mango-v4',
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
    await withdrawFeesToAdmin();
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
