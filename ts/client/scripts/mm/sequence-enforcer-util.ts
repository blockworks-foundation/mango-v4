import { BN } from '@coral-xyz/anchor';
import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import { createHash } from 'crypto';

export const seqEnforcerProgramIds = {
  devnet: new PublicKey('FBngRHN4s5cmHagqy3Zd6xcK3zPJBeX5DixtHFbBhyCn'),
  testnet: new PublicKey('FThcgpaJM8WiEbK5rw3i31Ptb8Hm4rQ27TrhfzeR1uUy'),
  'mainnet-beta': new PublicKey('GDDMwNyyx8uB6zrqwBFHjLLG3TBYk2F8Az4yrQC5RzMp'),
};

export function makeInitSequenceEnforcerAccountIx(
  account: PublicKey,
  ownerPk: PublicKey,
  bump: number,
  sym: string,
  cluster: string,
): TransactionInstruction {
  const keys = [
    { isSigner: false, isWritable: true, pubkey: account },
    { isSigner: true, isWritable: true, pubkey: ownerPk },
    { isSigner: false, isWritable: false, pubkey: SystemProgram.programId },
  ];

  const variant = createHash('sha256')
    .update('global:initialize')
    .digest()
    .slice(0, 8);

  const bumpData = new BN(bump).toBuffer('le', 1);
  const strLen = new BN(sym.length).toBuffer('le', 4);
  const symEncoded = Buffer.from(sym);

  const data = Buffer.concat([variant, bumpData, strLen, symEncoded]);

  return new TransactionInstruction({
    keys,
    data,
    programId: seqEnforcerProgramIds[cluster],
  });
}

export function makeCheckAndSetSequenceNumberIx(
  sequenceAccount: PublicKey,
  ownerPk: PublicKey,
  seqNum: number,
  cluster,
): TransactionInstruction {
  const keys = [
    { isSigner: false, isWritable: true, pubkey: sequenceAccount },
    { isSigner: true, isWritable: false, pubkey: ownerPk },
  ];
  const variant = createHash('sha256')
    .update('global:check_and_set_sequence_number')
    .digest()
    .slice(0, 8);

  const seqNumBuffer = new BN(seqNum).toBuffer('le', 8);
  const data = Buffer.concat([variant, seqNumBuffer]);
  return new TransactionInstruction({
    keys,
    data,
    programId: seqEnforcerProgramIds[cluster],
  });
}
