import * as anchor from '@coral-xyz/anchor';
import { AnchorProvider, BN, Program } from '@coral-xyz/anchor';
import NodeWallet from '@coral-xyz/anchor/dist/cjs/nodewallet';
import * as spl from '@solana/spl-token';
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  Keypair,
} from '@solana/web3.js';
import { MangoV4 } from '../target/types/mango_v4';
import ProgramKeypair from '../target/deploy/mango_v4-keypair.json';
import {
  Group,
  MangoAccount,
  MangoClient,
  StubOracle,
  U64_MAX_BN,
} from '../ts/client/src/index';
import { assert } from 'chai';
import {
  PerpMarketIndex,
  PerpOrderSide,
  PerpOrderType,
} from '../ts/client/src/accounts/perp';

enum MINTS {
  USDC = 'USDC',
  BTC = 'BTC',
}
const NUM_USERS = 4;
const PROGRAM_ID = '4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg';

interface TestUser {
  keypair: anchor.web3.Keypair;
  tokenAccounts: spl.AccountInfo[];
  mangoAccount: MangoAccount;
  client: MangoClient;
}

async function createMints(
  program: anchor.Program<MangoV4>,
  payer: anchor.web3.Keypair,
  admin,
): Promise<Partial<Record<keyof typeof MINTS, spl.Token>>> {
  const mints: spl.Token[] = [];
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
  const mintsMap = {
    USDC: mints[0],
    BTC: mints[1],
  };

  return mintsMap;
}

async function createUsers(
  mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>,
  payer: anchor.web3.Keypair,
  provider: anchor.Provider,
  group: Group,
  connection: Connection,
  programId: PublicKey,
): Promise<TestUser[]> {
  const users: TestUser[] = [];
  for (let i = 0; i < NUM_USERS; i++) {
    const user = anchor.web3.Keypair.generate();

    await provider.connection.requestAirdrop(
      user.publicKey,
      LAMPORTS_PER_SOL * 1000,
    );

    const tokenAccounts: spl.AccountInfo[] = [];
    for (const mintKey in mintsMap) {
      const mint: spl.Token = mintsMap[mintKey];
      const tokenAccount = await mint.getOrCreateAssociatedAccountInfo(
        user.publicKey,
      );
      await mint.mintTo(tokenAccount.address, payer, [], 1_000_000_000_000_000);
      tokenAccounts.push(tokenAccount);
    }

    const client = await MangoClient.connect(
      new anchor.AnchorProvider(
        connection,
        new NodeWallet(user),
        AnchorProvider.defaultOptions(),
      ),
      'devnet',
      programId,
      { idsSource: 'get-program-accounts' },
    );

    const mangoAccount = await client.getOrCreateMangoAccount(group);
    await mangoAccount!.reload(client);

    console.log('created user ' + i);
    users.push({
      keypair: user,
      tokenAccounts: tokenAccounts,
      client,
      mangoAccount: mangoAccount!,
    });
  }

  return users;
}

describe('mango-v4', () => {
  const programId = new PublicKey(PROGRAM_ID);
  // Configure the client to use the local cluster.
  const envProvider = anchor.AnchorProvider.env();
  anchor.setProvider(envProvider);
  const envProviderWallet = envProvider.wallet;
  const envProviderPayer = (envProviderWallet as NodeWallet).payer;

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    process.env.ANCHOR_PROVIDER_URL!,
    options.commitment,
  );

  const program = anchor.workspace.MangoV4 as Program<MangoV4>;
  let users: TestUser[] = [];

  let mintsMap: Partial<Record<keyof typeof MINTS, spl.Token>>;

  let group: Group;
  let usdcOracle: StubOracle;
  let btcOracle: StubOracle;
  let envClient: MangoClient;

  it('Initialize group and users', async () => {
    console.log(`provider ${envProviderWallet.publicKey.toString()}`);
    console.log(`program id ${programId.toString()}`);

    mintsMap = await createMints(program, envProviderPayer, envProviderWallet);

    const groupNum = 0;
    const insuranceMintPk = mintsMap['USDC']!.publicKey;
    const adminPk = envProviderWallet.publicKey;

    // Passing devnet as the cluster here - client cannot accept localnet
    // I think this is only for getting the serum market though?
    envClient = await MangoClient.connect(envProvider, 'devnet', programId, {
      idsSource: 'get-program-accounts',
    });
    await envClient.groupCreate(groupNum, true, 1, insuranceMintPk);
    group = await envClient.getGroupForCreator(adminPk, groupNum);

    users = await createUsers(
      mintsMap,
      envProviderPayer,
      envProvider,
      group,
      connection,
      programId,
    );

    assert.strictEqual(group.groupNum, groupNum);
    assert.deepEqual(group.admin, adminPk);
  });

  it('Create stub oracles and register tokens', async () => {
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

    const banks = await envClient.getBanksForGroup(group);
    assert.equal(banks.length, 2, 'Two banks present');

    assert.equal(banks[0].name, 'USDC', 'USDC bank present');
    assert.equal(banks[0].tokenIndex, 0, 'USDC bank token index set');
    assert.equal(banks[0].uiDeposits(), 0, 'USDC bank has zero deposits');
    assert.equal(banks[0].uiBorrows(), 0, 'USDC bank has zero borrows');

    assert.equal(banks[1].name, 'BTC', 'BTC bank present');
    assert.equal(banks[1].tokenIndex, 1, 'BTC bank token index set');
    assert.equal(banks[1].uiDeposits(), 0, 'BTC bank has zero deposits');
    assert.equal(banks[1].uiBorrows(), 0, 'BTC bank has zero borrows');
  });

  it('Create perp market', async () => {
    await envClient.perpCreateMarket(
      group,
      btcOracle.publicKey,
      0,
      'BTC-PERP',
      0.1,
      6,
      10,
      100,
      0.975,
      0.95,
      1.025,
      1.05,
      0.012,
      0.0002,
      0.0,
      0,
      0.05,
      0.05,
      100,
      true,
      true,
      1_000_000,
      0,
      0,
      0,
    );
    await group.reloadAll(envClient);

    const perps = await envClient.perpGetMarkets(group);
    assert.equal(perps.length, 1, 'One perp market present');
    assert.equal(perps[0].name, 'BTC-PERP', 'Name is correct');
    assert.equal(perps[0].perpMarketIndex, 0, 'Index is correct');
    assert.equal(
      perps[0].oracle.toBase58(),
      btcOracle.publicKey.toBase58(),
      'Oracle is correct',
    );
  });

  it('Edit perp market', async () => {
    let btcPerp = (await envClient.perpGetMarkets(group))[0];

    assert.closeTo(
      btcPerp.makerFee.toNumber(),
      0.0002,
      0.00001,
      'Maker fee before',
    );
    assert.closeTo(
      btcPerp.takerFee.toNumber(),
      0.0,
      0.00001,
      'Taker fee before',
    );

    await envClient.perpEditMarket(
      group,
      btcPerp.perpMarketIndex,
      btcPerp.oracle,
      0.1,
      btcPerp.baseDecimals,
      btcPerp.maintAssetWeight.toNumber(),
      btcPerp.initAssetWeight.toNumber(),
      btcPerp.maintLiabWeight.toNumber(),
      btcPerp.initLiabWeight.toNumber(),
      btcPerp.liquidationFee.toNumber(),
      0.0004, // Changed
      0.0005, // Changed
      0.0,
      btcPerp.minFunding.toNumber(),
      btcPerp.maxFunding.toNumber(),
      btcPerp.impactQuantity.toNumber(),
      true,
      true,
      1_000_000,
      0,
      0,
    );

    await group.reloadAll(envClient);

    btcPerp = (await envClient.perpGetMarkets(group))[0];

    assert.closeTo(
      btcPerp.makerFee.toNumber(),
      0.0004,
      0.00001,
      'Maker fee changed',
    );
    assert.closeTo(
      btcPerp.takerFee.toNumber(),
      0.0005,
      0.00001,
      'Taker fee changed',
    );
  });

  it('Deposit & Withdraw tokens', async () => {
    const client = await MangoClient.connect(
      new anchor.AnchorProvider(
        connection,
        new NodeWallet(users[0].keypair),
        options,
      ),
      'devnet',
      programId,
      { idsSource: 'get-program-accounts' },
    );

    const mangoAccount = await client.getOrCreateMangoAccount(group);
    await mangoAccount!.reload(client);

    await client.tokenDeposit(
      group,
      mangoAccount!,
      mintsMap.USDC!.publicKey,
      100.5,
    );
    await mangoAccount!.reload(client);

    await client.tokenDeposit(
      group,
      mangoAccount!,
      mintsMap.BTC!.publicKey,
      50.5,
    );
    await mangoAccount!.reload(client);

    const banks = await envClient.getBanksForGroup(group);
    assert.equal(
      mangoAccount!.getTokenBalanceUi(banks[0]),
      100.5,
      'USDC balance reflects deposit',
    );
    assert.equal(
      mangoAccount!.getTokenBalanceUi(banks[1]),
      50.5,
      'BTC balance reflects deposit',
    );

    await client.tokenWithdraw(
      group,
      mangoAccount!,
      mintsMap.USDC!.publicKey,
      100,
      false,
    );
    await mangoAccount!.reload(client);

    await client.tokenWithdraw(
      group,
      mangoAccount!,
      mintsMap.BTC!.publicKey,
      50,
      false,
    );
    await mangoAccount!.reload(client);

    assert.equal(
      mangoAccount!.getTokenBalanceUi(banks[0]),
      0.5,
      'USDC balance reflects withdrawal',
    );
    assert.equal(
      mangoAccount!.getTokenBalanceUi(banks[1]),
      0.5,
      'BTC balance reflects withdrawal',
    );
  });

  it('Place, cancel & match perp orders', async () => {
    const makerClient = users[2].client;
    const takerClient = users[3].client;

    const makerAccount = users[2].mangoAccount;
    const takerAccount = users[3].mangoAccount;
    await makerAccount.reload(makerClient);
    await takerAccount.reload(takerClient);

    // Expand each account to allow for perps
    await makerClient.expandMangoAccount(group, makerAccount, 8, 8, 8, 8);
    await takerClient.expandMangoAccount(group, takerAccount, 8, 8, 8, 8);

    // Give each client some cash
    await makerClient.tokenDeposit(
      group,
      makerAccount,
      mintsMap.USDC!.publicKey,
      1000,
    );
    await makerAccount.reload(makerClient);

    await takerClient.tokenDeposit(
      group,
      takerAccount,
      mintsMap.USDC!.publicKey,
      1000,
    );
    await takerAccount.reload(takerClient);

    // Set price
    await envClient.stubOracleSet(group, btcOracle.publicKey, 100);

    btcOracle = (
      await envClient.getStubOracle(group, mintsMap['BTC']!.publicKey)
    )[0];

    assert.equal(
      btcOracle.price.toNumber(),
      100.0,
      'Oracle price has been set',
    );

    // Maker places passive order
    await makerClient.perpPlaceOrder(
      group,
      makerAccount,
      0 as PerpMarketIndex,
      PerpOrderSide.bid,
      99.0,
      2.0,
      100_000,
      0,
      PerpOrderType.limit,
      0,
      20,
    );

    await makerClient.perpConsumeAllEvents(group, 0 as PerpMarketIndex);

    await makerAccount.reload(makerClient);
    let makerOrders = await makerAccount.loadPerpOpenOrdersForMarket(
      makerClient,
      group,
      0 as PerpMarketIndex,
    );
    assert.equal(makerOrders.length, 1, 'Maker has one open order');
    assert.equal(makerOrders[0].uiPrice, 99.0, 'Price correct');
    assert.equal(makerOrders[0].uiSize, 2.0, 'Size correct');
    assert.equal(makerOrders[0].side, PerpOrderSide.bid, 'Side correct');

    // Partial fill by taker
    await takerClient.perpPlaceOrder(
      group,
      takerAccount,
      0 as PerpMarketIndex,
      PerpOrderSide.ask,
      98.0,
      1.0,
      100_000,
      0,
      PerpOrderType.limit,
      0,
      20,
    );

    await takerClient.perpConsumeAllEvents(group, 0 as PerpMarketIndex);
    await takerAccount.reload(takerClient);
    await makerAccount.reload(takerClient);

    makerOrders = await makerAccount.loadPerpOpenOrdersForMarket(
      makerClient,
      group,
      0 as PerpMarketIndex,
    );
    assert.equal(makerOrders.length, 1, 'Maker still has one open order');
    assert.equal(makerOrders[0].uiSize, 1.0, 'Size reduced');

    const makerPerps = makerAccount.perpActive();
    assert.equal(makerPerps.length, 1, 'Maker has perp position');
    assert.equal(makerPerps[0].marketIndex, 0, 'Market index matches');
    assert.isTrue(
      makerPerps[0].basePositionLots.eq(new anchor.BN(10000)),
      'base position correct',
    );

    const takerPerps = takerAccount.perpActive();
    assert.equal(takerPerps.length, 1, 'Taker has perp position');
    assert.equal(takerPerps[0].marketIndex, 0, 'Market index matches');
    assert.isTrue(
      takerPerps[0].basePositionLots.eq(new anchor.BN(-10000)),
      'base position correct',
    );

    // Cancel remaining order
    await makerClient.perpCancelAllOrders(
      group,
      makerAccount,
      0 as PerpMarketIndex,
      20,
    );
    makerOrders = await makerAccount.loadPerpOpenOrdersForMarket(
      makerClient,
      group,
      0 as PerpMarketIndex,
    );
    assert.equal(makerOrders.length, 0, 'Maker orders have been canceled');
  });

  it('Test perp settle pnl & settle fees', async () => {
    const settlerClient = users[1].client;
    const makerClient = users[2].client;
    const takerClient = users[3].client;

    const settlerAccount = users[1].mangoAccount;
    const makerAccount = users[2].mangoAccount;
    const takerAccount = users[3].mangoAccount;

    await settlerAccount.reload(settlerClient);
    await makerAccount.reload(makerClient);
    await takerAccount.reload(takerClient);

    // Set higher price
    await envClient.stubOracleSet(group, btcOracle.publicKey, 200);

    btcOracle = (
      await envClient.getStubOracle(group, mintsMap['BTC']!.publicKey)
    )[0];

    assert.equal(
      btcOracle.price.toNumber(),
      200.0,
      'Oracle price has been set',
    );

    const banks = await envClient.getBanksForGroup(group);
    await makerAccount.reload(makerClient);
    await takerAccount.reload(takerClient);

    assert.equal(
      makerAccount.getTokenBalanceUi(banks[0]),
      1000,
      'Maker balance before',
    );
    assert.equal(
      takerAccount.getTokenBalanceUi(banks[0]),
      1000,
      'Taker balance before',
    );

    // Settle the PnL. Taker account is short, so it is unprofitable
    await settlerClient.perpSettlePnl(
      group,
      makerAccount,
      takerAccount,
      settlerAccount,
      0 as PerpMarketIndex,
    );

    await makerAccount.reload(makerClient);
    await takerAccount.reload(takerClient);
    await settlerAccount.reload(settlerClient);

    assert.closeTo(
      makerAccount.getTokenBalanceUi(banks[0]),
      1100,
      1,
      'Maker balance has profit',
    );
    assert.closeTo(
      takerAccount.getTokenBalanceUi(banks[0]),
      900,
      1,
      'Taker balance has loss',
    );
    assert.closeTo(
      settlerAccount.getTokenBalanceUi(banks[0]),
      1,
      0.001,
      'Settler balance has extra fee',
    );

    // Set even higher price
    await envClient.stubOracleSet(group, btcOracle.publicKey, 250);

    // Settle the fees
    let btcPerp = (await envClient.perpGetMarkets(group))[0];
    assert.closeTo(
      btcPerp.feesAccrued.toNumber(),
      89100,
      0.1,
      'Fees accrued by contract',
    );
    assert.equal(
      btcPerp.feesSettled.toNumber(),
      0,
      'Fees settled by contract is 0',
    );

    await envClient.perpSettleFees(
      group,
      takerAccount,
      0 as PerpMarketIndex,
      U64_MAX_BN,
    );

    btcPerp = (await envClient.perpGetMarkets(group))[0];
    assert.equal(
      btcPerp.feesAccrued.toNumber(),
      0,
      'Fees accrued by contract is 0',
    );
    assert.closeTo(
      btcPerp.feesSettled.toNumber(),
      89100,
      0.1,
      'Fees accrued by contract have been settled',
    );

    await takerAccount.reload(takerClient);
    assert.closeTo(
      takerAccount.getTokenBalanceUi(banks[0]),
      898.95,
      0.01,
      'Taker balance has more loss',
    );
  });

  it('liquidate token with token', async () => {
    const clientA = users[0].client;
    const clientB = users[1].client;

    const mangoAccountA = users[0].mangoAccount;
    const mangoAccountB = users[1].mangoAccount;
    await mangoAccountA!.reload(clientA);
    await mangoAccountB!.reload(clientB);

    await envClient.stubOracleSet(group, btcOracle.publicKey, 100);

    // Initialize liquidator
    await clientA.tokenDeposit(
      group,
      mangoAccountA!,
      mintsMap.USDC!.publicKey,
      1000,
    );
    await clientA.tokenDeposit(
      group,
      mangoAccountA!,
      mintsMap.BTC!.publicKey,
      100,
    );

    // Deposit collateral
    await clientB.tokenDeposit(
      group,
      mangoAccountB!,
      mintsMap.BTC!.publicKey,
      100,
    );
    await mangoAccountB!.reload(clientB);

    // Borrow
    await clientB.tokenWithdraw(
      group,
      mangoAccountB!,
      mintsMap.USDC!.publicKey,
      200,
      true,
    );

    // Set price so health is below maintenance
    await envClient.stubOracleSet(group, btcOracle.publicKey, 1);

    await mangoAccountB!.reload(clientB);

    await clientA.liqTokenWithToken(
      group,
      mangoAccountA!,
      mangoAccountB!,
      mintsMap.BTC!.publicKey,
      mintsMap.USDC!.publicKey,
      1000,
    );
  });

  it('update index and rate', async () => {
    let bank = (await envClient.getBanksForGroup(group))[0];
    const lastUpdated = bank.indexLastUpdated;

    await envClient.updateIndexAndRate(group, mintsMap.USDC!.publicKey);

    bank = (await envClient.getBanksForGroup(group))[0];
    assert.isTrue(
      bank.indexLastUpdated > lastUpdated,
      'Index timestamp updated',
    );
  });

  it('calculates entry and break even price correctly', async () => {
    const { client: clientA, mangoAccount: accountA } = users[2];
    const { client: clientB, mangoAccount: accountB } = users[3];

    await accountA!.reload(clientA);
    await accountB!.reload(clientB);

    const btcPerp = (await envClient.perpGetMarkets(group))[0];
    const positionA = accountA.getPerpPosition(btcPerp.perpMarketIndex)!;
    const positionB = accountB.getPerpPosition(btcPerp.perpMarketIndex)!;

    assert.equal(positionA.getBasePositionUi(btcPerp), 1, 'Position is long');
    assert.equal(positionB.getBasePositionUi(btcPerp), -1, 'Position is short');

    assert.isTrue(
      positionA.getEntryPrice(btcPerp).eq(new BN(99.0)),
      'long entry price matches',
    );
    assert.isTrue(
      positionB.getEntryPrice(btcPerp).eq(new BN(99.0)),
      'short entry price matches',
    );

    assert.isTrue(
      positionA.getBreakEvenPrice(btcPerp).eq(new BN(99.0)),
      'long break even price matches',
    );
    assert.isTrue(
      positionB.getBreakEvenPrice(btcPerp).eq(new BN(99.0)),
      'short break even price matches',
    );
  });
});
