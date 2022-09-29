import { AnchorProvider } from '@project-serum/anchor';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
  AccountMeta,
  AddressLookupTableAccount,
  MessageV0,
  PublicKey,
  Signer,
  SystemProgram,
  TransactionInstruction,
  VersionedTransaction,
} from '@solana/web3.js';
import BN from 'bn.js';
import { Bank, QUOTE_DECIMALS } from './accounts/bank';
import { Group } from './accounts/group';
import { I80F48 } from './accounts/I80F48';
import { MangoAccount, Serum3Orders } from './accounts/mangoAccount';
import { PerpMarket } from './accounts/perp';

export const U64_MAX_BN = new BN('18446744073709551615');
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

export function debugHealthAccounts(
  group: Group,
  mangoAccount: MangoAccount,
  publicKeys: PublicKey[],
) {
  const banks = new Map(
    Array.from(group.banksMapByName.values()).map((banks: Bank[]) => [
      banks[0].publicKey.toBase58(),
      `${banks[0].name} bank`,
    ]),
  );
  const oracles = new Map(
    Array.from(group.banksMapByName.values()).map((banks: Bank[]) => [
      banks[0].oracle.toBase58(),
      `${banks[0].name} oracle`,
    ]),
  );
  const serum3 = new Map(
    mangoAccount.serum3Active().map((serum3: Serum3Orders) => {
      const serum3Market = Array.from(
        group.serum3MarketsMapByExternal.values(),
      ).find((serum3Market) => serum3Market.marketIndex === serum3.marketIndex);
      if (!serum3Market) {
        throw new Error(
          `Serum3Orders for non existent market with market index ${serum3.marketIndex}`,
        );
      }
      return [serum3.openOrders.toBase58(), `${serum3Market.name} spot oo`];
    }),
  );
  const perps = new Map(
    Array.from(group.perpMarketsMapByName.values()).map(
      (perpMarket: PerpMarket) => [
        perpMarket.publicKey.toBase58(),
        `${perpMarket.name} perp market`,
      ],
    ),
  );

  publicKeys.map((pk) => {
    if (banks.get(pk.toBase58())) {
      console.log(banks.get(pk.toBase58()));
    }
    if (oracles.get(pk.toBase58())) {
      console.log(oracles.get(pk.toBase58()));
    }
    if (serum3.get(pk.toBase58())) {
      console.log(serum3.get(pk.toBase58()));
    }
    if (perps.get(pk.toBase58())) {
      console.log(perps.get(pk.toBase58()));
    }
  });
}

export async function findOrCreate<T>(
  entityName: string,
  findMethod: (...x: any) => any,
  findArgs: any[],
  createMethod: (...x: any) => any,
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

export function toNative(uiAmount: number, decimals: number): I80F48 {
  return I80F48.fromNumber(uiAmount).mul(
    I80F48.fromNumber(Math.pow(10, decimals)),
  );
}

export function toNativeDecimals(amount: number, decimals: number): BN {
  return new BN(Math.trunc(amount * Math.pow(10, decimals)));
}

export function toUiDecimals(
  amount: I80F48 | number,
  decimals: number,
): number {
  amount = amount instanceof I80F48 ? amount.toNumber() : amount;
  return amount / Math.pow(10, decimals);
}

export function toUiDecimalsForQuote(amount: I80F48 | number): number {
  amount = amount instanceof I80F48 ? amount.toNumber() : amount;
  return amount / Math.pow(10, QUOTE_DECIMALS);
}

export function toU64(amount: number, decimals: number): BN {
  const bn = toNativeDecimals(amount, decimals).toString();
  console.log('bn', bn);

  return new BN(bn);
}

export function nativeI80F48ToUi(amount: I80F48, decimals: number): I80F48 {
  return amount.div(I80F48.fromNumber(Math.pow(10, decimals)));
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
  // TODO: remove use of any when possible in future
  vTx.sign([
    ((provider as AnchorProvider).wallet as any).payer as Signer,
    ...additionalSigners,
  ]);
  return vTx;
}
