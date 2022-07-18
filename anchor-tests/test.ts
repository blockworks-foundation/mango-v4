import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { MangoV4 } from '../target/types/mango_v4';
import * as spl from '@solana/spl-token';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import * as assert from 'assert';
import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../ts/client/src/index';

enum MINTS {
  USDC = 'USDC',
  BTC = 'BTC',
}

const NUM_USERS = 4;

async function createMints(
  program: anchor.Program<MangoV4>,
  payer: anchor.web3.Keypair,
  admin,
) {
  let mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>;
  let mints: spl.Token[] = [];
  for (let i = 0; i < 2; i++) {
    mints.push(
      await spl.Token.createMint(
        program.provider.connection,
        payer,
        admin.publicKey,
        admin.publicKey,
        6,
        spl.TOKEN_PROGRAM_ID,
      ),
    );
  }
  mintsMap = {
    USDC: mints[0],
    BTC: mints[1],
  };

  return mintsMap;
}

async function createUsers(
  mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>,
  payer: anchor.web3.Keypair,
) {
  let users: { key: anchor.web3.Keypair; tokenAccounts: spl.AccountInfo[] }[] =
    [];
  for (let i = 0; i < NUM_USERS; i++) {
    let user = anchor.web3.Keypair.generate();

    let tokenAccounts: spl.AccountInfo[] = [];
    for (let mintKey in mintsMap) {
      let mint: spl.Token = mintsMap[mintKey];
      let tokenAccount = await mint.getOrCreateAssociatedAccountInfo(
        user.publicKey,
      );
      await mint.mintTo(tokenAccount.address, payer, [], 1_000_000_000_000_000);
      tokenAccounts.push(tokenAccount);
    }
    console.log('created user ' + i);
    users.push({ key: user, tokenAccounts: tokenAccounts });
  }

  return users;
}

describe('mango-v4', () => {
  let programId = new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD');
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  let providerWallet = provider.wallet;
  let providerPayer = (providerWallet as NodeWallet).payer;

  const program = anchor.workspace.MangoV4 as Program<MangoV4>;
  let users: { key: anchor.web3.Keypair; tokenAccounts: spl.AccountInfo[] }[] =
    [];

  let mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>;

  it('Is initialized!', async () => {
    console.log(`provider ${providerWallet.publicKey.toString()}`);

    mintsMap = await createMints(program, providerPayer, providerWallet);
    users = await createUsers(mintsMap, providerPayer);

    console.log(`users ${users.map((e) => e.key.publicKey.toString())}`);
  });

  it('test_basic', async () => {
    let groupNum = 0;
    let testing = 0;
    let insuranceMintPk = mintsMap['USDC']!.publicKey;
    const adminPk = providerWallet.publicKey;

    // Passing devnet as the cluster here - client cannot accept localnet
    // I think this is only for getting the serum market though?
    const client = await MangoClient.connect(provider, 'devnet', programId);
    await client.groupCreate(
      groupNum,
      testing === 1 ? true : false,
      insuranceMintPk,
    );
    let group = await client.getGroupForAdmin(adminPk, groupNum);

    await client.stubOracleCreate(group, mintsMap['USDC']!.publicKey, 1.0);
    const usdcOracle = (
      await client.getStubOracle(group, mintsMap['USDC']!.publicKey)
    )[0];

    await client.tokenRegister(
      group,
      mintsMap['USDC']!.publicKey,
      usdcOracle.publicKey,
      0.1,
      0, // tokenIndex
      'USDC',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      1.5,
      0.0005,
      1.5,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reloadAll(client);

    const mangoAccount = await client.getOrCreateMangoAccount(
      group,
      providerWallet.publicKey,
      0,
      'my_mango_account',
    );
    await mangoAccount.reload(client, group);

    await client.tokenDeposit(group, mangoAccount, 'USDC', 50);
    await mangoAccount.reload(client, group);
  });
});
