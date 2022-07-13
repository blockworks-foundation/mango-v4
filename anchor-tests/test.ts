import * as anchor from "@project-serum/anchor";
import { Program, Spl, SplToken } from "@project-serum/anchor";
import { MangoV4 } from "../target/types/mango_v4";
import * as spl from '@solana/spl-token';
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import * as assert from 'assert';

describe("mango-v5", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  let wallet = provider.wallet;
  let payer = (wallet as NodeWallet).payer;

  const program = anchor.workspace.MangoV4 as Program<MangoV4>;
  let users: { key: anchor.web3.Keypair, tokenAccounts: spl.AccountInfo[] }[] = [];
  let mints: spl.Token[] = [];

  it("Is initialized!", async () => {

    console.log(wallet.publicKey.toString());

    // Create mints
    for (let i = 0; i < 2; i++) {
      mints.push(await spl.Token.createMint(
        program.provider.connection,
        payer,
        wallet.publicKey,
        wallet.publicKey,
        6,
        spl.TOKEN_PROGRAM_ID
      ))
    }

    // Create users
    for (let i = 0; i < 4; i++) {
      let user = anchor.web3.Keypair.generate();

      let tokenAccounts: spl.AccountInfo[] = []
      for (let mint of mints) {
        let tokenAccount = await mint.getOrCreateAssociatedAccountInfo(
          user.publicKey
        )
        await mint.mintTo(tokenAccount.address, payer, [], 1_000_000_000_000_000);
        tokenAccounts.push(tokenAccount);
      }
      console.log('created user ' + i);
      users.push({ "key": user, "tokenAccounts": tokenAccounts })
    }

    console.log(users.map(e => e.key.publicKey.toString()));
  });

  // it("test_basic", async () => {
  //   let result = await program.methods.groupCreate(0,0).accounts({'group': anchor.web3.Keypair.generate().publicKey, 'admin': wallet.publicKey, 'insuranceMint': mints[0].publicKey, 'insuranceVault': anchor.web3.Keypair.generate().publicKey, payer: wallet.publicKey }).rpc();
  // });
});