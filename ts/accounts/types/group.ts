import {
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import { MangoClient } from '../../client';

export class Group {
  static from(publicKey: PublicKey, obj: { admin: PublicKey }): Group {
    return new Group(publicKey, obj.admin);
  }

  constructor(public publicKey: PublicKey, public admin: PublicKey) {}
}

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
