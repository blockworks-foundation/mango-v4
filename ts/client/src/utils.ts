import { AnchorProvider } from '@coral-xyz/anchor';
import {
  AddressLookupTableAccount,
  MessageV0,
  PublicKey,
  Signer,
  SystemProgram,
  TransactionInstruction,
  VersionedTransaction,
} from '@solana/web3.js';
import BN from 'bn.js';
import { Bank } from './accounts/bank';
import { I80F48 } from './numbers/I80F48';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from './utils/spl';

///
/// numeric helpers
///
export const U64_MAX_BN = new BN('18446744073709551615');
export const I64_MAX_BN = new BN('9223372036854775807').toTwos(64);

export function bpsToDecimal(bps: number): number {
  return bps / 10000;
}

export function percentageToDecimal(percentage: number): number {
  return percentage / 100;
}

export function toNativeI80F48ForQuote(uiAmount: number): I80F48 {
  return I80F48.fromNumber(uiAmount * Math.pow(10, 6));
}

export function toNativeI80F48(uiAmount: number, decimals: number): I80F48 {
  return I80F48.fromNumber(uiAmount * Math.pow(10, decimals));
}

export function toNative(uiAmount: number, decimals: number): BN {
  return new BN((uiAmount * Math.pow(10, decimals)).toFixed(0));
}

export function toNativeSellPerBuyTokenPrice(
  price: number,
  sellBank: Bank,
  buyBank: Bank,
): number {
  return price * Math.pow(10, sellBank.mintDecimals - buyBank.mintDecimals);
}

export function toUiSellPerBuyTokenPrice(
  price: number,
  sellBank: Bank,
  buyBank: Bank,
): number {
  return toUiDecimals(price, sellBank.mintDecimals - buyBank.mintDecimals);
}

export function toUiDecimals(
  nativeAmount: BN | I80F48 | number,
  decimals: number,
): number {
  // TODO: remove BN and upgrade to bigint https://github.com/solana-labs/solana/issues/27440
  if (nativeAmount instanceof BN) {
    nativeAmount = I80F48.fromU64(nativeAmount);
  }
  if (nativeAmount instanceof I80F48) {
    return nativeAmount
      .div(I80F48.fromNumber(Math.pow(10, decimals)))
      .toNumber();
  }
  return nativeAmount / Math.pow(10, decimals);
}

export const QUOTE_DECIMALS = 6;

export function toUiDecimalsForQuote(
  nativeAmount: BN | I80F48 | number,
): number {
  return toUiDecimals(nativeAmount, QUOTE_DECIMALS);
}

export function toUiI80F48(nativeAmount: I80F48, decimals: number): I80F48 {
  return nativeAmount.div(I80F48.fromNumber(Math.pow(10, decimals)));
}

export function roundTo5(number): number {
  if (number < 1) {
    const numString = number.toString();
    const nonZeroIndex = numString.search(/[1-9]/);
    if (nonZeroIndex === -1 || nonZeroIndex >= numString.length - 5) {
      return number;
    }
    return Number(numString.slice(0, nonZeroIndex + 5));
  } else if (number < 10) {
    return (
      Math.floor(number) +
      Number((number % 1).toString().padEnd(10, '0').slice(0, 6))
    );
  } else if (number < 100) {
    return (
      Math.floor(number) +
      Number((number % 1).toString().padEnd(10, '0').slice(0, 5))
    );
  } else if (number < 1000) {
    return (
      Math.floor(number) +
      Number((number % 1).toString().padEnd(10, '0').slice(0, 4))
    );
  } else if (number < 10000) {
    return (
      Math.floor(number) +
      Number((number % 1).toString().padEnd(10, '0').slice(0, 3))
    );
  }
  return Math.round(number);
}

///

export async function buildFetch(): Promise<
  (
    input: RequestInfo | URL,
    init?: RequestInit | undefined,
  ) => Promise<Response>
> {
  let fetch = globalThis?.fetch;
  if (!fetch && process?.versions?.node) {
    fetch = (await import('node-fetch')).default;
  }
  return fetch;
}

///

///
/// web3js extensions
///

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
  allowOwnerOffCurve = true,
  programId = TOKEN_PROGRAM_ID,
  associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID,
): Promise<PublicKey> {
  if (!allowOwnerOffCurve && !PublicKey.isOnCurve(owner.toBuffer()))
    throw new Error('TokenOwnerOffCurve!');

  const [address] = await PublicKey.findProgramAddress(
    [owner.toBuffer(), programId.toBuffer(), mint.toBuffer()],
    associatedTokenProgramId,
  );

  return address;
}

export async function createAssociatedTokenAccountIdempotentInstruction(
  payer: PublicKey,
  owner: PublicKey,
  mint: PublicKey,
): Promise<TransactionInstruction> {
  const account = await getAssociatedTokenAddress(mint, owner);
  return new TransactionInstruction({
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: account, isSigner: false, isWritable: true },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      {
        pubkey: SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    programId: ASSOCIATED_TOKEN_PROGRAM_ID,
    data: Buffer.from([0x1]),
  });
}

export async function buildVersionedTx(
  provider: AnchorProvider,
  ix: TransactionInstruction[],
  additionalSigners: Signer[] = [],
  alts: AddressLookupTableAccount[] = [],
): Promise<VersionedTransaction> {
  const message = MessageV0.compile({
    payerKey: (provider as AnchorProvider).wallet.publicKey,
    instructions: ix,
    recentBlockhash: (await provider.connection.getLatestBlockhash()).blockhash,
    addressLookupTableAccounts: alts,
  });
  const vTx = new VersionedTransaction(message);
  vTx.sign([
    ((provider as AnchorProvider).wallet as any).payer as Signer,
    ...additionalSigners,
  ]);
  return vTx;
}

///
/// ts extension
///

// https://stackoverflow.com/questions/70261755/user-defined-type-guard-function-and-type-narrowing-to-more-specific-type/70262876#70262876
export declare abstract class As<Tag extends keyof never> {
  private static readonly $as$: unique symbol;
  private [As.$as$]: Record<Tag, true>;
}

export function deepClone<T>(obj: T, hash = new WeakMap()): T {
  // Handle non-object types and functions
  if (typeof obj !== 'object' || obj === null) return obj;

  // Handle circular references
  if (hash.has(obj)) return hash.get(obj) as T;

  let result: any;

  if (obj instanceof Map) {
    result = new Map();
    hash.set(obj, result);
    obj.forEach((value, key) => {
      result.set(deepClone(key, hash), deepClone(value, hash));
    });
  } else if (obj instanceof Set) {
    result = new Set();
    hash.set(obj, result);
    for (const item of obj) {
      result.add(deepClone(item, hash));
    }
  } else if (Array.isArray(obj)) {
    result = [];
    hash.set(obj, result);
    obj.forEach((item, index) => {
      result[index] = deepClone(item, hash);
    });
  } else {
    const prototype = Object.getPrototypeOf(obj);
    result = Object.create(prototype);
    hash.set(obj, result);
    for (const key of Object.keys(obj)) {
      result[key] = deepClone((obj as any)[key], hash);
    }
  }

  return result;
}

export const tryStringify = (val: any): string | null => {
  try {
    return JSON.stringify(val);
  } catch (e) {
    return null;
  }
};
