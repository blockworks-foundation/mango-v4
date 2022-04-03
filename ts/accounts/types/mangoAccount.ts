import {
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import { Bank } from './bank';
import { I80F48, I80F48Dto } from './I80F48';
import { MangoClient } from '../../client';
import { BN } from '@project-serum/anchor';

export class MangoAccount {
  public tokens: TokenAccount[];

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      owner: PublicKey;
      delegate: PublicKey;
      tokens: unknown;
      serum3: Object;
      perps: unknown;
      beingLiquidated: number;
      isBankrupt: number;
      accountNum: number;
      bump: number;
      reserved: number[];
    },
  ) {
    return new MangoAccount(
      publicKey,
      obj.group,
      obj.owner,
      obj.delegate,
      obj.tokens as { values: TokenAccountDto[] },
      obj.serum3,
      obj.perps,
      obj.beingLiquidated,
      obj.isBankrupt,
      obj.accountNum,
      obj.bump,
      obj.reserved,
    );
  }

  constructor(
    public publicKey: PublicKey,
    group: PublicKey,
    owner: PublicKey,
    delegate: PublicKey,
    tokens: { values: TokenAccountDto[] },
    serum3: Object,
    perps: unknown,
    beingLiquidated: number,
    isBankrupt: number,
    accountNum: number,
    bump: number,
    reserved: number[],
  ) {
    this.tokens = tokens.values.map((dto) => TokenAccount.from(dto));
  }

  find(tokenIndex: number): TokenAccount | undefined {
    return this.tokens.find((ta) => ta.tokenIndex == tokenIndex);
  }

  getNativeDeposit(bank: Bank): I80F48 {
    const ta = this.find(bank.tokenIndex);
    return bank.depositIndex.mul(ta?.indexedValue!);
  }

  getNativeBorrow(bank: Bank): I80F48 {
    const ta = this.find(bank.tokenIndex);
    return bank.borrowIndex.mul(ta?.indexedValue!);
  }
}

export class TokenAccount {
  static from(dto: TokenAccountDto) {
    return new TokenAccount(
      I80F48.from(dto.indexedValue),
      dto.tokenIndex,
      dto.inUseCount,
    );
  }

  constructor(
    public indexedValue: I80F48,
    public tokenIndex: number,
    public inUseCount: number,
  ) {}
}

export class TokenAccountDto {
  constructor(
    public indexedValue: I80F48Dto,
    public tokenIndex: number,
    public inUseCount: number,
    public reserved: number[],
  ) {}
}

export async function createMangoAccount(
  client: MangoClient,
  groupPk: PublicKey,
  ownerPk: PublicKey,
  payer: Keypair,
): Promise<void> {
  const tx = new Transaction();
  const signers = [payer];
  const ix = await createMangoAccountIx(client, groupPk, ownerPk, payer);
  tx.add(ix);
  await client.program.provider.send(tx, signers);
}

export async function createMangoAccountIx(
  client: MangoClient,
  groupPk: PublicKey,
  ownerPk: PublicKey,
  payer: Keypair,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .createAccount(11)
    .accounts({
      group: groupPk,
      owner: ownerPk,
      payer: payer.publicKey,
    })
    .signers([payer])
    .instruction();
}

export async function closeMangoAccount(
  client: MangoClient,
  accountPk: PublicKey,
  ownerPk: PublicKey,
) {
  const tx = new Transaction();
  const ix = await closeMangoAccountIx(client, accountPk, ownerPk);
  tx.add(ix);
  await client.program.provider.send(tx);
}

export async function closeMangoAccountIx(
  client: MangoClient,
  accountPk: PublicKey,
  ownerPk: PublicKey,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .closeAccount()
    .accounts({
      account: accountPk,
      owner: ownerPk,
      solDestination: ownerPk,
    })
    .instruction();
}

export async function getMangoAccount(
  client: MangoClient,
  address: PublicKey,
): Promise<MangoAccount> {
  return MangoAccount.from(
    address,
    await client.program.account.mangoAccount.fetch(address),
  );
}

export async function getMangoAccountsForGroup(
  client: MangoClient,
  groupPk: PublicKey,
): Promise<MangoAccount[]> {
  return (
    await client.program.account.mangoAccount.all([
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
    ])
  ).map((pa) => MangoAccount.from(pa.publicKey, pa.account));
}

export async function getMangoAccountsForGroupAndOwner(
  client: MangoClient,
  groupPk: PublicKey,
  ownerPk: PublicKey,
): Promise<MangoAccount[]> {
  return (
    await client.program.account.mangoAccount.all([
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: ownerPk.toBase58(),
          offset: 40,
        },
      },
    ])
  ).map((pa) => MangoAccount.from(pa.publicKey, pa.account));
}

export async function deposit(
  client: MangoClient,
  groupPk: PublicKey,
  mangoAccountPk: PublicKey,
  bankPk: PublicKey,
  vaultPk: PublicKey,
  tokenAccountPk: PublicKey,
  oraclePk: PublicKey,
  ownerPk: PublicKey,
  amount: number,
): Promise<void> {
  const tx = new Transaction();
  const ix = await depositIx(
    client,
    groupPk,
    mangoAccountPk,
    bankPk,
    vaultPk,
    tokenAccountPk,
    oraclePk,
    ownerPk,
    amount,
  );
  tx.add(ix);
  await client.program.provider.send(tx);
}

export async function depositIx(
  client: MangoClient,
  groupPk: PublicKey,
  mangoAccountPk: PublicKey,
  bankPk: PublicKey,
  vaultPk: PublicKey,
  tokenAccountPk: PublicKey,
  oraclePk: PublicKey,
  ownerPk: PublicKey,
  amount: number,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .deposit(new BN(amount))
    .accounts({
      group: groupPk,
      account: mangoAccountPk,
      bank: bankPk,
      vault: vaultPk,
      tokenAccount: tokenAccountPk,
      tokenAuthority: ownerPk,
    })
    .remainingAccounts([
      { pubkey: bankPk, isWritable: false, isSigner: false },
      { pubkey: oraclePk, isWritable: false, isSigner: false },
    ])
    .instruction();
}

export async function withdraw(
  client: MangoClient,
  groupPk: PublicKey,
  mangoAccountPk: PublicKey,
  bankPk: PublicKey,
  vaultPk: PublicKey,
  tokenAccountPk: PublicKey,
  oraclePk: PublicKey,
  ownerPk: PublicKey,
  amount: number,
  allowBorrow: boolean,
): Promise<void> {
  const tx = new Transaction();
  const ix = await withdrawIx(
    client,
    groupPk,
    mangoAccountPk,
    bankPk,
    vaultPk,
    tokenAccountPk,
    oraclePk,
    ownerPk,
    amount,
    allowBorrow,
  );
  tx.add(ix);
  await client.program.provider.send(tx);
}

export async function withdrawIx(
  client: MangoClient,
  groupPk: PublicKey,
  mangoAccountPk: PublicKey,
  bankPk: PublicKey,
  vaultPk: PublicKey,
  tokenAccountPk: PublicKey,
  oraclePk: PublicKey,
  ownerPk: PublicKey,
  amount: number,
  allowBorrow: boolean,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .withdraw(new BN(amount), allowBorrow)
    .accounts({
      group: groupPk,
      account: mangoAccountPk,
      bank: bankPk,
      vault: vaultPk,
      tokenAccount: tokenAccountPk,
      tokenAuthority: ownerPk,
    })
    .remainingAccounts([
      { pubkey: bankPk, isWritable: false, isSigner: false },
      { pubkey: oraclePk, isWritable: false, isSigner: false },
    ])
    .instruction();
}
