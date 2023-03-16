import { AnchorProvider } from '@coral-xyz/anchor';
import { IdlCoder } from '@coral-xyz/anchor/dist/cjs/coder/borsh/idl';
import {
  AddressLookupTableAccount,
  Connection,
  MessageV0,
  PublicKey,
  Signer,
  SystemProgram,
  TransactionInstruction,
  VersionedTransaction,
} from '@solana/web3.js';
import BN from 'bn.js';

import { decode } from '@coral-xyz/anchor/dist/cjs/utils/bytes/base64';
import { MangoClient } from './client';
import { IDL } from './mango_v4';
import { I80F48 } from './numbers/I80F48';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from './utils/spl';

///
/// numeric helpers
///
export const U64_MAX_BN = new BN('18446744073709551615');
export const I64_MAX_BN = new BN('9223372036854775807').toTwos(64);

export function bpsToDecimal(bps: number): number {
  return bps / 1000;
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
  allowOwnerOffCurve = false,
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

///
/// Anchor / Solana
///

export function groupTxLogsByIx(txLogs: string[]): string[][] {
  const logsGroupedByIx: string[][] = [];
  let currentIx: string[] = [];
  for (const log of txLogs) {
    if (log.indexOf('invoke') > -1) {
      currentIx.length !== 0 ? logsGroupedByIx.push(currentIx) : false;
      currentIx = [];
    }
    currentIx.push(log);
  }
  currentIx.length !== 0 ? logsGroupedByIx.push(currentIx) : false;

  return logsGroupedByIx;
}

export function extractReturnValuesForSolanaTxLogs<T>(
  programId: PublicKey,
  ixName: string,
  txLogs: string[],
): T[] {
  const logsGroupedByIx = groupTxLogsByIx(txLogs);
  const ixNamePrefix = `Program log: Instruction: ${
    ixName.charAt(0).toUpperCase() + ixName.slice(1)
  }`;

  const filteredlogsGroupedByIx = logsGroupedByIx.filter(
    (ixLogs) => ixLogs[1].indexOf(ixNamePrefix) > -1,
  );
  const logMessages = filteredlogsGroupedByIx.flat();

  const returnPrefix = `Program return: ${programId} `;
  const returnLogs = logMessages.filter((l) => l.startsWith(returnPrefix));
  if (!returnLogs) {
    throw new Error('Expected return log');
  }

  return returnLogs.map((returnLog) => {
    const returnData = decode(returnLog.slice(returnPrefix.length));
    const returnType = (
      IDL.instructions.find((ix) => ix.name === ixName) as any
    ).returns;
    if (!returnType) {
      throw new Error('Expected return type');
    }
    const coder = IdlCoder.fieldLayout(
      { type: returnType },
      Array.from([...(IDL.accounts ?? []), ...(IDL.types ?? [])]),
    );
    return coder.decode(returnData);
  });
}

export async function extractReturnValuesForSolanaTx<T>(
  client: MangoClient,
  ixName: string,
  signature: string,
): Promise<T[]> {
  // we need a 'confirmed' level connection
  const conn = new Connection(
    client.program.provider.connection.rpcEndpoint,
    'confirmed',
  );
  const tx = await conn.getTransaction(signature, {
    maxSupportedTransactionVersion: 1,
  });

  if (!tx?.meta?.logMessages) {
    throw new Error('Tx meta logMessages is null or undefined');
  }

  return extractReturnValuesForSolanaTxLogs(
    client.programId,
    ixName,
    tx.meta.logMessages,
  );
}
