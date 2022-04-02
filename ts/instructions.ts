import { BN, ProgramAccount } from '@project-serum/anchor';
import { TransactionInstruction } from '@solana/web3.js';
import { Transaction } from '@solana/web3.js';
import { Keypair, PublicKey, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import * as bs58 from 'bs58';
import { MangoClient } from './client';
import { Bank, Group, MangoAccount } from './types';
import { debugAccountMetas } from './utils';

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
): Promise<ProgramAccount<Group>[]> {
  return (await client.program.account.group.all([
    {
      memcmp: {
        bytes: bs58.encode(adminPk.toBuffer()),
        offset: 8,
      },
    },
  ])) as ProgramAccount<Group>[];
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
          bytes: bs58.encode(groupPk.toBuffer()),
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
          bytes: bs58.encode(groupPk.toBuffer()),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(mintPk.toBuffer()),
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
          bytes: bs58.encode(groupPk.toBuffer()),
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
          bytes: bs58.encode(groupPk.toBuffer()),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(ownerPk.toBuffer()),
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

// export async function serum3_register_market(
//   client: MangoClient,
//   groupPk: PublicKey,
//   adminPk: PublicKey,
//   serumProgramPk: PublicKey,
//   serumMarketExternalPk: PublicKey,
//   quoteBankPk: PublicKey,
//   baseBankPk: PublicKey,
//   ownerPk: PublicKey,
//   amount: number,
//   allowBorrow: boolean,
// ): Promise<TransactionInstruction> {
//   return await client.program.methods
//     .withdraw(new BN(amount), allowBorrow)
//     .accounts({
//       group: groupPk,
//       account: mangoAccountPk,
//       bank: bankPk,
//       vault: vaultPk,
//       tokenAccount: tokenAccountPk,
//       tokenAuthority: ownerPk,
//     })
//     .remainingAccounts([
//       { pubkey: bankPk, isWritable: false, isSigner: false },
//       { pubkey: oraclePk, isWritable: false, isSigner: false },
//     ])
//     .instruction();
// }
