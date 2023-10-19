import { Program, Provider, web3 } from '@coral-xyz/anchor';
import { Idl } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';

export const DEFAULT_VSR_ID = new web3.PublicKey(
  '4Q6WW2ouZ6V3iaNm56MTd5n2tnTm4C5fiH8miFHnAFHo',
);

export class VsrClient {
  constructor(public program: Program<Idl>, public devnet?: boolean) {}

  static async connect(
    provider: Provider,
    programId: web3.PublicKey,
    devnet?: boolean,
  ): Promise<VsrClient> {
    const idl = await Program.fetchIdl(new PublicKey(DEFAULT_VSR_ID), provider);
    return new VsrClient(
      new Program<Idl>(idl as Idl, programId, provider),
      devnet,
    );
  }
}
