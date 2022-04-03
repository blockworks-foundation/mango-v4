import { BN } from '@project-serum/anchor';
import {
  Keypair,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import * as bs58 from 'bs58';
import { MangoClient } from './client';
import { Bank, Group, MangoAccount, Serum3Market, StubOracle } from './types';
import { I80F48 } from './I80F48';

//
// group
//

export async function createGroup(
  client: MangoClient,
  adminPk: PublicKey,
  payer: Keypair,
): Promise<void> {
  const tx = new Transaction();
  const signers = [payer];
  const ix = await createGroupIx(client, adminPk, payer);
  tx.add(ix);
  await client.program.provider.send(tx, signers);
}

export async function createGroupIx(
  client: MangoClient,
  adminPk: PublicKey,
  payer: Keypair,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .createGroup()
    .accounts({
      admin: adminPk,
      payer: payer.publicKey,
    })
    .signers([payer])
    .instruction();
}

export async function getGroupForAdmin(
  client: MangoClient,
  adminPk: PublicKey,
): Promise<Group[]> {
  return (
    await client.program.account.group.all([
      {
        memcmp: {
          bytes: adminPk.toBase58(),
          offset: 8,
        },
      },
    ])
  ).map((tuple) => Group.from(tuple.publicKey, tuple.account));
}

//
// token / bank
//

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

//
// mango account
//

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

//
// deposit & withdraw
//

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

//
// Serum3 instructions
//

export async function serum3RegisterMarket(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  serumProgramPk: PublicKey,
  serumMarketExternalPk: PublicKey,
  quoteBankPk: PublicKey,
  baseBankPk: PublicKey,
  payer: Keypair,
  marketIndex: number,
): Promise<void> {
  const tx = new Transaction();
  const ix = await serum3RegisterMarketIx(
    client,
    groupPk,
    adminPk,
    serumProgramPk,
    serumMarketExternalPk,
    quoteBankPk,
    baseBankPk,
    payer,
    marketIndex,
  );
  tx.add(ix);
  await client.program.provider.send(tx, [payer]);
}

export async function serum3RegisterMarketIx(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  serumProgramPk: PublicKey,
  serumMarketExternalPk: PublicKey,
  quoteBankPk: PublicKey,
  baseBankPk: PublicKey,
  payer: Keypair,
  marketIndex: number,
): Promise<TransactionInstruction> {
  return await client.program.methods
    .serum3RegisterMarket(marketIndex)
    .accounts({
      group: groupPk,
      admin: adminPk,
      serumProgram: serumProgramPk,
      serumMarketExternal: serumMarketExternalPk,
      quoteBank: quoteBankPk,
      baseBank: baseBankPk,
      payer: payer.publicKey,
    })
    .instruction();
}

export async function getSerum3MarketForBaseAndQuote(
  client: MangoClient,
  groupPk: PublicKey,
  baseTokenIndex: number,
  quoteTokenIndex: number,
): Promise<Serum3Market[]> {
  const bbuf = Buffer.alloc(2);
  bbuf.writeUInt16LE(baseTokenIndex);

  const qbuf = Buffer.alloc(2);
  qbuf.writeUInt16LE(quoteTokenIndex);

  const bumpfbuf = Buffer.alloc(1);
  bumpfbuf.writeUInt8(255);

  return (
    await client.program.account.serum3Market.all([
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 106,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(qbuf),
          offset: 108,
        },
      },
    ])
  ).map((tuple) => Serum3Market.from(tuple.publicKey, tuple.account));
}

//
// Oracle
//

export async function createStubOracle(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  tokenMintPk: PublicKey,
  payer: Keypair,
  staticPrice: number,
): Promise<void> {
  return await client.program.methods
    .createStubOracle({ val: I80F48.fromNumber(staticPrice).getData() })
    .accounts({
      group: groupPk,
      admin: adminPk,
      tokenMint: tokenMintPk,
      payer: payer.publicKey,
    })
    .signers([payer])
    .rpc();
}

export async function setStubOracle(
  client: MangoClient,
  groupPk: PublicKey,
  adminPk: PublicKey,
  tokenMintPk: PublicKey,
  payer: Keypair,
  staticPrice: number,
): Promise<void> {
  return await client.program.methods
    .setStubOracle({ val: new BN(staticPrice) })
    .accounts({
      group: groupPk,
      admin: adminPk,
      tokenMint: tokenMintPk,
      payer: payer.publicKey,
    })
    .signers([payer])
    .rpc();
}

export async function getStubOracleForGroupAndMint(
  client: MangoClient,
  groupPk: PublicKey,
  mintPk: PublicKey,
): Promise<StubOracle[]> {
  return (
    await client.program.account.stubOracle.all([
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
  ).map((pa) => StubOracle.from(pa.publicKey, pa.account));
}
