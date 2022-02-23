import { Program, Provider } from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';
import { MangoV4, IDL } from './mango_v4';

export const MANGO_V4_ID = new PublicKey(
  'Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS',
);

export class MangoClient {
  constructor(
    public program: Program<MangoV4>,
    public devnet?: boolean,
  ) {}

  static async connect(
    provider: Provider,
    devnet?: boolean,
  ): Promise<MangoClient> {
    // alternatively we could fetch from chain
    // const idl = await Program.fetchIdl(MANGO_V$_ID, provider);
    const idl = IDL;

    return new MangoClient(
      new Program<MangoV4>(
        idl as MangoV4,
        MANGO_V4_ID,
        provider,
      ),
      devnet,
    );
  }
}
