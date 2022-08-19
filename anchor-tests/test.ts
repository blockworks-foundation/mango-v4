import * as anchor from '@project-serum/anchor';
import { AnchorProvider, Program } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import * as spl from '@solana/spl-token';
import { Connection, LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js';
import { MangoV4 } from '../target/types/mango_v4';
import { Group, MangoClient, StubOracle } from '../ts/client/src/index';

enum MINTS {
  USDC = 'USDC',
  BTC = 'BTC',
}
const NUM_USERS = 2;

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
  provider: anchor.Provider,
) {
  let users: {
    keypair: anchor.web3.Keypair;
    tokenAccounts: spl.AccountInfo[];
  }[] = [];
  for (let i = 0; i < NUM_USERS; i++) {
    let user = anchor.web3.Keypair.generate();

    await provider.connection.requestAirdrop(
      user.publicKey,
      LAMPORTS_PER_SOL * 1000,
    );

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
    users.push({ keypair: user, tokenAccounts: tokenAccounts });
  }

  return users;
}

describe('mango-v4', () => {
  let programId = new PublicKey('4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg');
  // Configure the client to use the local cluster.
  const envProvider = anchor.AnchorProvider.env();
  anchor.setProvider(envProvider);
  let envProviderWallet = envProvider.wallet;
  let envProviderPayer = (envProviderWallet as NodeWallet).payer;

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    process.env.ANCHOR_PROVIDER_URL!,
    options.commitment,
  );

  const program = anchor.workspace.MangoV4 as Program<MangoV4>;
  let users: {
    keypair: anchor.web3.Keypair;
    tokenAccounts: spl.AccountInfo[];
  }[] = [];

  let mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>;

  let group: Group;
  let usdcOracle: StubOracle;
  let btcOracle: StubOracle;
  let envClient: MangoClient;

  it('Initialize group and users', async () => {
    console.log(`provider ${envProviderWallet.publicKey.toString()}`);

    mintsMap = await createMints(program, envProviderPayer, envProviderWallet);
    users = await createUsers(mintsMap, envProviderPayer, envProvider);

    const groupNum = 0;
    const insuranceMintPk = mintsMap['USDC']!.publicKey;
    const adminPk = envProviderWallet.publicKey;

    // Passing devnet as the cluster here - client cannot accept localnet
    // I think this is only for getting the serum market though?
    envClient = await MangoClient.connect(envProvider, 'devnet', programId);
    await envClient.groupCreate(groupNum, false, 1, insuranceMintPk);
    group = await envClient.getGroupForCreator(adminPk, groupNum);

    await envClient.stubOracleCreate(group, mintsMap['USDC']!.publicKey, 1.0);
    usdcOracle = (
      await envClient.getStubOracle(group, mintsMap['USDC']!.publicKey)
    )[0];

    await envClient.tokenRegister(
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
    await group.reloadAll(envClient);

    await envClient.stubOracleCreate(group, mintsMap['BTC']!.publicKey, 100.0);
    btcOracle = (
      await envClient.getStubOracle(group, mintsMap['BTC']!.publicKey)
    )[0];

    await envClient.tokenRegister(
      group,
      mintsMap['BTC']!.publicKey,
      btcOracle.publicKey,
      0.1,
      1, // tokenIndex
      'BTC',
      0.01,
      0.4,
      0.07,
      0.8,
      0.9,
      0.88,
      0.0005,
      1.5,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    );
    await group.reloadAll(envClient);

    await envClient.perpCreateMarket(
      group,
      btcOracle.publicKey,
      0,
      'BTC-PERP',
      0.1,
      1,
      6,
      1,
      10,
      100,
      0.975,
      0.95,
      1.025,
      1.05,
      0.012,
      0.0002,
      0.0,
      0.05,
      0.05,
      100,
    );
    await group.reloadAll(envClient);
  });

  it('Basic', async () => {
    const client = await MangoClient.connect(
      new anchor.AnchorProvider(
        connection,
        new NodeWallet(users[0].keypair),
        options,
      ),
      'devnet',
      programId,
    );

    const mangoAccount = await client.getOrCreateMangoAccount(
      group,
      users[0].keypair.publicKey,
    );
    await mangoAccount.reload(client, group);

    await client.tokenDeposit(group, mangoAccount, 'USDC', 100.5);
    await mangoAccount.reload(client, group);

    await client.tokenDeposit(group, mangoAccount, 'BTC', 50.5);
    await mangoAccount.reload(client, group);

    await client.tokenWithdraw(group, mangoAccount, 'USDC', 100, false);
    await mangoAccount.reload(client, group);

    await client.tokenWithdraw(group, mangoAccount, 'BTC', 50, false);
    await mangoAccount.reload(client, group);
  });

  it('liquidate token and token', async () => {
    const clientA = await MangoClient.connect(
      new anchor.AnchorProvider(
        connection,
        new NodeWallet(users[0].keypair),
        options,
      ),
      'devnet',
      programId,
    );
    const clientB = await MangoClient.connect(
      new anchor.AnchorProvider(
        connection,
        new NodeWallet(users[1].keypair),
        options,
      ),
      'devnet',
      programId,
    );

    const mangoAccountA = await clientA.getOrCreateMangoAccount(
      group,
      users[0].keypair.publicKey,
    );
    await mangoAccountA.reload(clientA, group);

    const mangoAccountB = await clientB.getOrCreateMangoAccount(
      group,
      users[1].keypair.publicKey,
    );
    await mangoAccountB.reload(clientB, group);

    await envClient.stubOracleSet(group, btcOracle.publicKey, 100);

    // Initialize liquidator
    await clientA.tokenDeposit(group, mangoAccountA, 'USDC', 1000);
    await clientA.tokenDeposit(group, mangoAccountA, 'BTC', 100);

    // Deposit collateral
    await clientB.tokenDeposit(group, mangoAccountB, 'BTC', 100);
    await mangoAccountB.reload(clientB, group);
    // // Borrow
    await clientB.tokenWithdraw(group, mangoAccountB, 'USDC', 200, true);
    // // Set price so health is below maintanence
    await envClient.stubOracleSet(group, btcOracle.publicKey, 1);

    await mangoAccountB.reload(clientB, group);

    await clientA.liqTokenWithToken(
      group,
      mangoAccountA,
      mangoAccountB,
      'BTC',
      'USDC',
      1000,
    );
  });

  it('update index and rate', async () => {
    envClient.updateIndexAndRate(group, 'USDC');
  });
});
