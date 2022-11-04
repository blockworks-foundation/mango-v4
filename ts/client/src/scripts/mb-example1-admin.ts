import { AnchorProvider, Wallet } from '@project-serum/anchor';
import {
  AddressLookupTableProgram,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../accounts/group';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { buildVersionedTx } from '../utils';

const MAINNET_MINTS = new Map([
  ['USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'],
  ['USDT', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB'],
  ['BTC', '9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E'], // Wrapped Bitcoin (Sollet)
  ['ETH', '7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs'], // Ether (Portal)
  ['soETH', '2FPyTwcZLUg1MDrwsyoP4D6s1tM7hAkHYRjkNb5w6Pxk'], // Wrapped Ethereum (Sollet)
  ['SOL', 'So11111111111111111111111111111111111111112'], // Wrapped SOL
  ['MSOL', 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So'],
  ['MNGO', 'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac'],
]);
const MAINNET_ORACLES = new Map([
  ['USDT', '3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL'],
  ['BTC', 'GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'],
  ['ETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['soETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['SOL', 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'],
  ['MSOL', 'E4v1BBgoso9s64TQvmyownAVJbhbEPGyzA3qn4n46qj9'],
  ['MNGO', '79wm3jjcPr6RaNQ4DGvP5KxG1mNd3gEBsg6FsNVFezK4'],
]);

// External markets are matched with those in https://github.com/blockworks-foundation/mango-client-v3/blob/main/src/ids.json
// and verified to have best liquidity for pair on https://openserum.io/
const MAINNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', 'A8YFbxQYFVqKZaoYJLLUVcQiWP7G2MeEgW5wsAQgMvFw'],
  ['SOL/USDC', '9wFFyRfZBsuAha4YcuxcXLKwMxJR43S7fPfQLusDBzvT'],
]);

async function buildAdminClient(): Promise<[MangoClient, Keypair]> {
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  return [
    await MangoClient.connect(
      adminProvider,
      'mainnet-beta',
      MANGO_V4_ID['mainnet-beta'],
    ),
    admin,
  ];
}

async function buildUserClient(
  userKeypair: string,
): Promise<[MangoClient, Group, Keypair]> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(userKeypair, 'utf-8'))),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);

  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  return [client, group, user];
}

async function createGroup() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  console.log(`Creating Group...`);
  const insuranceMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  await client.groupCreate(2, true, 0, insuranceMint);
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`...registered group ${group.publicKey}`);
}

async function registerTokens() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroupForCreator(admin.publicKey, 2);

  console.log(`Creating USDC stub oracle...`);
  const usdcMainnetMint = new PublicKey(MAINNET_MINTS.get('USDC')!);
  await client.stubOracleCreate(group, usdcMainnetMint, 1.0);
  const usdcMainnetOracle = (
    await client.getStubOracle(group, usdcMainnetMint)
  )[0];
  console.log(`...created stub oracle ${usdcMainnetOracle.publicKey}`);

  console.log(`Registering USDC...`);
  await client.tokenRegister(
    group,
    usdcMainnetMint,
    usdcMainnetOracle.publicKey,
    0.1,
    0,
    'USDC',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    1,
    1,
    1,
    1,
    0,
  );

  console.log(`Registering USDT...`);
  const usdtMainnetMint = new PublicKey(MAINNET_MINTS.get('USDT')!);
  const usdtMainnetOracle = new PublicKey(MAINNET_ORACLES.get('USDT')!);
  await client.tokenRegister(
    group,
    usdtMainnetMint,
    usdtMainnetOracle,
    0.1,
    1,
    'USDT',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.95,
    0.9,
    1.05,
    1.1,
    0.025,
  );

  console.log(`Registering BTC...`);
  const btcMainnetMint = new PublicKey(MAINNET_MINTS.get('BTC')!);
  const btcMainnetOracle = new PublicKey(MAINNET_ORACLES.get('BTC')!);
  await client.tokenRegister(
    group,
    btcMainnetMint,
    btcMainnetOracle,
    0.1,
    2,
    'BTC',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering ETH...`);
  const ethMainnetMint = new PublicKey(MAINNET_MINTS.get('ETH')!);
  const ethMainnetOracle = new PublicKey(MAINNET_ORACLES.get('ETH')!);
  await client.tokenRegister(
    group,
    ethMainnetMint,
    ethMainnetOracle,
    0.1,
    3,
    'ETH',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering soETH...`);
  const soEthMainnetMint = new PublicKey(MAINNET_MINTS.get('soETH')!);
  const soEthMainnetOracle = new PublicKey(MAINNET_ORACLES.get('soETH')!);
  await client.tokenRegister(
    group,
    soEthMainnetMint,
    soEthMainnetOracle,
    0.1,
    4,
    'soETH',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering SOL...`);
  const solMainnetMint = new PublicKey(MAINNET_MINTS.get('SOL')!);
  const solMainnetOracle = new PublicKey(MAINNET_ORACLES.get('SOL')!);
  await client.tokenRegister(
    group,
    solMainnetMint,
    solMainnetOracle,
    0.1,
    5,
    'SOL',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  console.log(`Registering MSOL...`);
  const msolMainnetMint = new PublicKey(MAINNET_MINTS.get('MSOL')!);
  const msolMainnetOracle = new PublicKey(MAINNET_ORACLES.get('MSOL')!);
  await client.tokenRegister(
    group,
    msolMainnetMint,
    msolMainnetOracle,
    0.1,
    6,
    'MSOL',
    0.004,
    0.7,
    0.1,
    0.85,
    0.2,
    2.0,
    0.005,
    0.0005,
    0.9,
    0.8,
    1.1,
    1.2,
    0.05,
  );

  // log tokens/banks
  await group.reloadAll(client);
  for (const bank of await Array.from(group.banksMapByMint.values()).flat()) {
    console.log(`${bank.toString()}`);
  }
}

async function registerSerum3Markets() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroupForCreator(admin.publicKey, 2);

  // Bump version to 1 to unlock serum3 feature
  await client.groupEdit(
    group,
    group.admin,
    group.fastListingAdmin,
    undefined,
    1,
  );

  // Register BTC and SOL markets
  await client.serum3RegisterMarket(
    group,
    new PublicKey(MAINNET_SERUM3_MARKETS.get('BTC/USDC')!),
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('BTC')!)),
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('USDC')!)),
    0,
    'BTC/USDC',
  );
  await client.serum3RegisterMarket(
    group,
    new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('SOL')!)),
    group.getFirstBankByMint(new PublicKey(MAINNET_MINTS.get('USDC')!)),
    1,
    'SOL/USDC',
  );
}

async function createUser(userKeypair: string) {
  const result = await buildUserClient(userKeypair);
  const client = result[0];
  const group = result[1];
  const user = result[2];

  console.log(`Creating MangoAccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(group);
  if (!mangoAccount) {
    throw new Error(`MangoAccount not found for user ${user.publicKey}`);
  }

  console.log(`...created MangoAccount ${mangoAccount.publicKey.toBase58()}`);

  await client.tokenDeposit(
    group,
    mangoAccount,
    new PublicKey(MAINNET_MINTS.get('USDC')!),
    10,
  );
  await mangoAccount.reload(client);
  console.log(`...deposited 10 USDC`);

  await client.tokenDeposit(
    group,
    mangoAccount,
    new PublicKey(MAINNET_MINTS.get('SOL')!),
    1,
  );
  await mangoAccount.reload(client);
  console.log(`...deposited 1 SOL`);
}

async function expandMangoAccount(userKeypair: string) {
  const result = await buildUserClient(userKeypair);
  const client = result[0];
  const group = result[1];
  const user = result[2];

  const mangoAccounts = await client.getMangoAccountsForOwner(
    group,
    user.publicKey,
  );
  if (!mangoAccounts) {
    throw new Error(`MangoAccounts not found for user ${user.publicKey}`);
  }

  for (const mangoAccount of mangoAccounts) {
    console.log(`...found MangoAccount ${mangoAccount.publicKey.toBase58()}`);
    await client.expandMangoAccount(group, mangoAccount, 8, 2, 0, 0);
  }
}

async function placeSerum3TradeAndCancelIt(userKeypair: string) {
  const result = await buildUserClient(userKeypair);
  const client = result[0];
  const group = result[1];
  const user = result[2];

  const mangoAccounts = await client.getMangoAccountsForOwner(
    group,
    user.publicKey,
  );
  if (!mangoAccounts) {
    throw new Error(`MangoAccounts not found for user ${user.publicKey}`);
  }

  for (const mangoAccount of mangoAccounts) {
    console.log(`...found MangoAccount ${mangoAccount.publicKey.toBase58()}`);
    console.log(`...placing serum3 order`);
    await client.serum3PlaceOrder(
      group,
      mangoAccount,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
      Serum3Side.bid,
      1,
      1,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      Date.now(),
      10,
    );
    console.log(`...current own orders on OB`);
    let orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
      client,
      group,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
    );
    for (const order of orders) {
      console.log(order);
    }
    console.log(`...cancelling serum3 orders`);
    await client.serum3CancelAllOrders(
      group,
      mangoAccount,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
      10,
    );
    console.log(`...current own orders on OB`);
    orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
      client,
      group,
      new PublicKey(MAINNET_SERUM3_MARKETS.get('SOL/USDC')!),
    );
    for (const order of orders) {
      console.log(order);
    }
  }
}

async function createAndPopulateAlt() {
  const result = await buildAdminClient();
  const client = result[0];
  const admin = result[1];

  const group = await client.getGroupForCreator(admin.publicKey, 2);

  const connection = client.program.provider.connection;

  // Create ALT, and set to group at index 0
  if (group.addressLookupTables[0].equals(PublicKey.default)) {
    try {
      console.log(`ALT: Creating`);
      const createIx = AddressLookupTableProgram.createLookupTable({
        authority: admin.publicKey,
        payer: admin.publicKey,
        recentSlot: await connection.getSlot('finalized'),
      });
      const createTx = await buildVersionedTx(
        client.program.provider as AnchorProvider,
        [createIx[0]],
      );
      let sig = await connection.sendTransaction(createTx);
      console.log(
        `...created ALT ${createIx[1]} https://explorer.solana.com/tx/${sig}`,
      );

      console.log(`ALT: set at index 0 for group...`);
      sig = await client.altSet(group, createIx[1], 0);
      console.log(`...https://explorer.solana.com/tx/${sig}`);
    } catch (error) {
      console.log(error);
    }
  }

  // Extend using mango v4 relevant pub keys
  try {
    let bankAddresses = Array.from(group.banksMapByMint.values())
      .flat()
      .map((bank) => [bank.publicKey, bank.oracle, bank.vault])
      .flat()
      .concat(
        Array.from(group.banksMapByMint.values())
          .flat()
          .map((mintInfo) => mintInfo.publicKey),
      );

    let serum3MarketAddresses = Array.from(
      group.serum3MarketsMapByExternal.values(),
    )
      .flat()
      .map((serum3Market) => serum3Market.publicKey);

    let serum3ExternalMarketAddresses = Array.from(
      group.serum3ExternalMarketsMap.values(),
    )
      .flat()
      .map((serum3ExternalMarket) => [
        serum3ExternalMarket.publicKey,
        serum3ExternalMarket.bidsAddress,
        serum3ExternalMarket.asksAddress,
      ])
      .flat();

    let perpMarketAddresses = Array.from(
      group.perpMarketsMapByMarketIndex.values(),
    )
      .flat()
      .map((perpMarket) => [
        perpMarket.publicKey,
        perpMarket.oracle,
        perpMarket.bids,
        perpMarket.asks,
        perpMarket.eventQueue,
      ])
      .flat();

    async function extendTable(addresses: PublicKey[]) {
      await group.reloadAll(client);
      const alt =
        await client.program.provider.connection.getAddressLookupTable(
          group.addressLookupTables[0],
        );

      addresses = addresses.filter(
        (newAddress) =>
          alt.value?.state.addresses &&
          alt.value?.state.addresses.findIndex((addressInALt) =>
            addressInALt.equals(newAddress),
          ) === -1,
      );
      if (addresses.length === 0) {
        return;
      }
      const extendIx = AddressLookupTableProgram.extendLookupTable({
        lookupTable: group.addressLookupTables[0],
        payer: admin.publicKey,
        authority: admin.publicKey,
        addresses,
      });
      const extendTx = await buildVersionedTx(
        client.program.provider as AnchorProvider,
        [extendIx],
      );
      let sig = await client.program.provider.connection.sendTransaction(
        extendTx,
      );
      console.log(`https://explorer.solana.com/tx/${sig}`);
    }

    console.log(`ALT: extending using mango v4 relevant public keys`);
    await extendTable(bankAddresses);
    await extendTable(serum3MarketAddresses);
    await extendTable(serum3ExternalMarketAddresses);
    await extendTable(perpMarketAddresses);
  } catch (error) {
    console.log(error);
  }
}

async function main() {
  try {
    // await createGroup();
  } catch (error) {
    console.log(error);
  }
  try {
    // await registerTokens();
  } catch (error) {
    console.log(error);
  }
  try {
    // await registerSerum3Markets();
  } catch (error) {
    console.log(error);
  }
  try {
    // await createUser(process.env.MB_USER_KEYPAIR!);
    // await createUser(process.env.MB_USER2_KEYPAIR!);
    // await expandMangoAccount(process.env.MB_USER_KEYPAIR!);
    // await placeSerum3TradeAndCancelIt(process.env.MB_USER_KEYPAIR!);
  } catch (error) {
    console.log(error);
  }

  try {
    await createAndPopulateAlt();
  } catch (error) {}
}

try {
  main();
} catch (error) {
  console.log(error);
}
