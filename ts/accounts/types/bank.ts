import { I80F48, I80F48Dto } from './I80F48';
import {
  Keypair,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import { MangoClient } from '../../client';
import { debugAccountMetas } from '../../utils';

export class Bank {
  public depositIndex: I80F48;
  public borrowIndex: I80F48;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      mint: PublicKey;
      vault: PublicKey;
      oracle: PublicKey;
      depositIndex: I80F48Dto;
      borrowIndex: I80F48Dto;
      indexedTotalDeposits: I80F48Dto;
      indexedTotalBorrows: I80F48Dto;
      maintAssetWeight: I80F48Dto;
      initAssetWeight: I80F48Dto;
      maintLiabWeight: I80F48Dto;
      initLiabWeight: I80F48Dto;
      liquidationFee: I80F48Dto;
      dust: Object;
      tokenIndex: number;
    },
  ) {
    return new Bank(
      publicKey,
      obj.group,
      obj.mint,
      obj.vault,
      obj.oracle,
      obj.depositIndex,
      obj.borrowIndex,
      obj.indexedTotalDeposits,
      obj.indexedTotalBorrows,
      obj.maintAssetWeight,
      obj.initAssetWeight,
      obj.maintLiabWeight,
      obj.initLiabWeight,
      obj.liquidationFee,
      obj.dust,
      obj.tokenIndex,
    );
  }

  constructor(
    public publicKey: PublicKey,
    group: PublicKey,
    mint: PublicKey,
    public vault: PublicKey,
    public oracle: PublicKey,
    depositIndex: I80F48Dto,
    borrowIndex: I80F48Dto,
    indexedTotalDeposits: I80F48Dto,
    indexedTotalBorrows: I80F48Dto,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    liquidationFee: I80F48Dto,
    dust: Object,
    public tokenIndex: number,
  ) {
    this.depositIndex = I80F48.from(depositIndex);
    this.borrowIndex = I80F48.from(borrowIndex);
  }

  toString(): string {
    return `Bank ${
      this.tokenIndex
    } deposit index - ${this.depositIndex.toNumber()}, borrow index - ${this.borrowIndex.toNumber()}`;
  }
}

export async function registerToken(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  mintPk: PublicKey,
  oraclePk: PublicKey,
  payer: Keypair,
  tokenIndex: number,
): Promise<void> {
  const tx = new Transaction();
  const signers = [payer];
  const ix = await registerTokenIx(
    client,
    groupPk,
    adminPk,
    mintPk,
    oraclePk,
    payer,
    tokenIndex,
  );
  tx.add(ix);
  await client.program.provider.send(tx, signers);
}

export async function registerTokenIx(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  mintPk: PublicKey,
  oraclePk: PublicKey,
  payer: Keypair,
  tokenIndex: number,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .registerToken(tokenIndex, 0.8, 0.6, 1.2, 1.4, 0.02)
    .accounts({
      group: groupPk,
      admin: adminPk,
      mint: mintPk,
      oracle: oraclePk,
      payer: payer.publicKey,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .signers([payer])
    .instruction();
}

export async function getBank(
  client: MangoClient,
  address: PublicKey,
): Promise<Bank> {
  return Bank.from(address, await client.program.account.bank.fetch(address));
}

export async function getBanksForGroup(
  client: MangoClient,
  groupPk: PublicKey,
): Promise<Bank[]> {
  return (
    await client.program.account.bank.all([
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
    ])
  ).map((tuple) => Bank.from(tuple.publicKey, tuple.account));
}

export async function getBankForGroupAndMint(
  client: MangoClient,
  groupPk: PublicKey,
  mintPk: PublicKey,
): Promise<Bank[]> {
  return (
    await client.program.account.bank.all([
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: mintPk.toBase58(),
          offset: 40,
        },
      },
    ])
  ).map((tuple) => Bank.from(tuple.publicKey, tuple.account));
}
