import {
  getGovernanceProgramVersion,
  getInstructionDataFromBase64,
  getSignatoryRecordAddress,
  ProgramAccount,
  serializeInstructionToBase64,
  TokenOwnerRecord,
  VoteType,
  WalletSigner,
  withAddSignatory,
  withCreateProposal,
  withInsertTransaction,
  withSignOffProposal,
} from '@solana/spl-governance';
import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import { chunk } from 'lodash';
import { updateVoterWeightRecord } from './updateVoteWeightRecord';
import { VsrClient } from './voteStakeRegistryClient';
import { createComputeBudgetIx } from '../../src/utils/rpc';
import { sendSignAndConfirmTransactions } from '@blockworks-foundation/mangolana/lib/transactions';
import { SequenceType } from '@blockworks-foundation/mangolana/lib/globalTypes';

export const MANGO_MINT = 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac';
export const MANGO_REALM_PK = new PublicKey(
  'DPiH3H3c7t47BMxqTxLsuPQpEC6Kne8GA9VXbxpnZxFE',
);
export const MANGO_GOVERNANCE_PROGRAM = new PublicKey(
  'GqTPL6qRf5aUuqscLh8Rg2HTxPUXfhhAXDptTLhp1t2J',
);

export const createProposal = async (
  connection: Connection,
  wallet: WalletSigner,
  governance: PublicKey,
  tokenOwnerRecord: ProgramAccount<TokenOwnerRecord>,
  name: string,
  descriptionLink: string,
  proposalIndex: number,
  proposalInstructions: TransactionInstruction[],
  client: VsrClient,
  signOff: boolean,
) => {
  const instructions: TransactionInstruction[] = [];
  const walletPk = wallet.publicKey!;
  const governanceAuthority = walletPk;
  const signatory = walletPk;
  const payer = walletPk;

  // Changed this because it is misbehaving on my local validator setup.
  const programVersion = await getGovernanceProgramVersion(
    connection,
    MANGO_GOVERNANCE_PROGRAM,
  );

  // V2 Approve/Deny configuration
  const voteType = VoteType.SINGLE_CHOICE;
  const options = ['Approve'];
  const useDenyOption = true;

  const { updateVoterWeightRecordIx, voterWeightPk } =
    await updateVoterWeightRecord(
      client,
      tokenOwnerRecord.account.governingTokenOwner,
    );
  instructions.push(updateVoterWeightRecordIx);

  const proposalAddress = await withCreateProposal(
    instructions,
    MANGO_GOVERNANCE_PROGRAM,
    programVersion,
    MANGO_REALM_PK,
    governance,
    tokenOwnerRecord.pubkey,
    name,
    descriptionLink,
    new PublicKey(MANGO_MINT),
    governanceAuthority,
    proposalIndex,
    voteType,
    options,
    useDenyOption,
    payer,
    voterWeightPk,
  );

  await withAddSignatory(
    instructions,
    MANGO_GOVERNANCE_PROGRAM,
    programVersion,
    proposalAddress,
    tokenOwnerRecord.pubkey,
    governanceAuthority,
    signatory,
    payer,
  );

  const insertInstructions: TransactionInstruction[] = [];
  for (const i in proposalInstructions) {
    try {
      const instruction = getInstructionDataFromBase64(
        serializeInstructionToBase64(proposalInstructions[i]),
      );
      await withInsertTransaction(
        insertInstructions,
        MANGO_GOVERNANCE_PROGRAM,
        programVersion,
        governance,
        proposalAddress,
        tokenOwnerRecord.pubkey,
        governanceAuthority,
        Number(i),
        0,
        0,
        [instruction],
        payer,
      );
    } catch (e) {
      console.log(e, '@@@@@@@');
    }
  }
  if (signOff) {
    const signatoryRecordAddress = await getSignatoryRecordAddress(
      MANGO_GOVERNANCE_PROGRAM,
      proposalAddress,
      signatory,
    );
    withSignOffProposal(
      insertInstructions, // SingOff proposal needs to be executed after inserting instructions hence we add it to insertInstructions
      MANGO_GOVERNANCE_PROGRAM,
      programVersion,
      MANGO_REALM_PK,
      governance,
      proposalAddress,
      signatory,
      signatoryRecordAddress,
      undefined,
    );
  }

  const txChunks = chunk([...instructions, ...insertInstructions], 2);

  await sendSignAndConfirmTransactions({
    connection,
    wallet,
    transactionInstructions: txChunks.map((txChunk) => ({
      instructionsSet: [
        {
          signers: [],
          transactionInstruction: createComputeBudgetIx(80000),
        },
        ...txChunk.map((tx) => ({
          signers: [],
          transactionInstruction: tx,
        })),
      ],
      sequenceType: SequenceType.Sequential,
    })),
    config: {
      maxRetries: 5,
      autoRetry: true,
      maxTxesInBatch: 20,
      logFlowInfo: true,
    },
  });

  return proposalAddress;
};
