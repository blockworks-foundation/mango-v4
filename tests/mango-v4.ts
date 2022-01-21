import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { MangoV4 } from '../target/types/mango_v4';

describe('mango-v4', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.MangoV4 as Program<MangoV4>;

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
