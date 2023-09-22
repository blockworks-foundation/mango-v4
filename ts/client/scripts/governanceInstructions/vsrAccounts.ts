import { BN } from '@coral-xyz/anchor';
import { Mint } from '@solana/spl-token';
import { PublicKey } from '@solana/web3.js';
import { VsrClient } from './voteStakeRegistryClient';

export type TokenProgramAccount<T> = {
  publicKey: PublicKey;
  account: T;
};

export interface Voter {
  deposits: Deposit[];
  voterAuthority: PublicKey;
  registrar: PublicKey;
  //there are more fields but no use for them on ui yet
}

export interface votingMint {
  baselineVoteWeightScaledFactor: BN;
  digitShift: number;
  grantAuthority: PublicKey;
  lockupSaturationSecs: BN;
  maxExtraLockupVoteWeightScaledFactor: BN;
  mint: PublicKey;
}

export type LockupType = 'none' | 'monthly' | 'cliff' | 'constant' | 'daily';
export interface Registrar {
  governanceProgramId: PublicKey;
  realm: PublicKey;
  realmAuthority: PublicKey;
  realmGoverningTokenMint: PublicKey;
  votingMints: votingMint[];
  //there are more fields but no use for them on ui yet
}
interface LockupKind {
  none: object;
  daily: object;
  monthly: object;
  cliff: object;
  constant: object;
}
interface Lockup {
  endTs: BN;
  kind: LockupKind;
  startTs: BN;
}
export interface Deposit {
  allowClawback: boolean;
  amountDepositedNative: BN;
  amountInitiallyLockedNative: BN;
  isUsed: boolean;
  lockup: Lockup;
  votingMintConfigIdx: number;
}

export interface DepositWithMintAccount extends Deposit {
  mint: TokenProgramAccount<Mint>;
  index: number;
  available: BN;
  vestingRate: BN | null;
  currentlyLocked: BN;
  nextVestingTimestamp: BN | null;
  votingPower: BN;
  votingPowerBaseline: BN;
}

export const emptyPk = '11111111111111111111111111111111';

export const getRegistrarPDA = async (
  realmPk: PublicKey,
  mint: PublicKey,
  clientProgramId: PublicKey,
) => {
  const [registrar, registrarBump] = await PublicKey.findProgramAddress(
    [realmPk.toBuffer(), Buffer.from('registrar'), mint.toBuffer()],
    clientProgramId,
  );
  return {
    registrar,
    registrarBump,
  };
};

export const getVoterPDA = async (
  registrar: PublicKey,
  walletPk: PublicKey,
  clientProgramId: PublicKey,
) => {
  const [voter, voterBump] = await PublicKey.findProgramAddress(
    [registrar.toBuffer(), Buffer.from('voter'), walletPk.toBuffer()],
    clientProgramId,
  );

  return {
    voter,
    voterBump,
  };
};

export const getVoterWeightPDA = async (
  registrar: PublicKey,
  walletPk: PublicKey,
  clientProgramId: PublicKey,
) => {
  const [voterWeightPk, voterWeightBump] = await PublicKey.findProgramAddress(
    [
      registrar.toBuffer(),
      Buffer.from('voter-weight-record'),
      walletPk.toBuffer(),
    ],
    clientProgramId,
  );

  return {
    voterWeightPk,
    voterWeightBump,
  };
};

export const tryGetVoter = async (voterPk: PublicKey, client: VsrClient) => {
  try {
    const voter = await client?.program.account.voter.fetch(voterPk);
    return voter as unknown as Voter;
  } catch (e) {
    return null;
  }
};
export const tryGetRegistrar = async (
  registrarPk: PublicKey,
  client: VsrClient,
) => {
  try {
    const existingRegistrar = await client.program.account.registrar.fetch(
      registrarPk,
    );
    return existingRegistrar as unknown as Registrar;
  } catch (e) {
    return null;
  }
};
