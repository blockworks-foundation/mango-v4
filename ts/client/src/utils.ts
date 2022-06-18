import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  u64,
} from '@solana/spl-token';
import { AccountMeta, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

export const I64_MAX_BN = new BN('9223372036854775807').toTwos(64);

export function debugAccountMetas(ams: AccountMeta[]) {
  for (const am of ams) {
    console.log(
      `${am.pubkey.toBase58()}, isSigner: ${am.isSigner
        .toString()
        .padStart(5, ' ')}, isWritable - ${am.isWritable
        .toString()
        .padStart(5, ' ')}`,
    );
  }
}

export async function findOrCreate<T>(
  entityName: string,
  findMethod: Function,
  findArgs: any[],
  createMethod: Function,
  createArgs: any[],
): Promise<T> {
  let many: T[] = await findMethod(...findArgs);
  let one: T;
  if (many.length > 0) {
    one = many[0];
    return one;
  }
  await createMethod(...createArgs);
  many = await findMethod(...findArgs);
  one = many[0];
  return one;
}

/**
 * Get the address of the associated token account for a given mint and owner
 *
 * @param mint                     Token mint account
 * @param owner                    Owner of the new account
 * @param allowOwnerOffCurve       Allow the owner account to be a PDA (Program Derived Address)
 * @param programId                SPL Token program account
 * @param associatedTokenProgramId SPL Associated Token program account
 *
 * @return Address of the associated token account
 */
export async function getAssociatedTokenAddress(
  mint: PublicKey,
  owner: PublicKey,
  allowOwnerOffCurve = false,
  programId = TOKEN_PROGRAM_ID,
  associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID,
): Promise<PublicKey> {
  if (!allowOwnerOffCurve && !PublicKey.isOnCurve(owner.toBuffer()))
    throw new Error('TokenOwnerOffCurve');

  const [address] = await PublicKey.findProgramAddress(
    [owner.toBuffer(), programId.toBuffer(), mint.toBuffer()],
    associatedTokenProgramId,
  );

  return address;
}

export function toNativeDecimals(amount: number, decimals: number): BN {
  return new BN(Math.trunc(amount * Math.pow(10, decimals)));
}

export function toUiDecimals(amount: number, decimals: number): number {
  return amount / Math.pow(10, decimals);
}

export function toU64(amount: number, decimals): BN {
  const bn = toNativeDecimals(amount, decimals).toString();
  console.log('bn', bn);

  return new u64(bn);
}
