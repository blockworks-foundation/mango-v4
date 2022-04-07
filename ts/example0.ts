import { Provider, Wallet, web3 } from '@project-serum/anchor';
import { Market } from '@project-serum/serum';
import * as spl from '@solana/spl-token';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import {
  Bank,
  getBank,
  getBankForGroupAndMint,
  getMintInfoForTokenIndex,
  registerToken,
} from './accounts/types/bank';
import { createGroup, getGroupForAdmin, Group } from './accounts/types/group';
import {
  createMangoAccount,
  deposit,
  getMangoAccount,
  getMangoAccountsForGroupAndOwner,
  MangoAccount,
  withdraw,
} from './accounts/types/mangoAccount';
import {
  createStubOracle,
  getStubOracleForGroupAndMint,
  StubOracle,
} from './accounts/types/oracle';
import {
  getSerum3MarketForBaseAndQuote,
  serum3CreateOpenOrders,
  Serum3Market,
  Serum3OrderType,
  serum3PlaceOrder,
  serum3RegisterMarket,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from './accounts/types/serum3';
import { MangoClient } from './client';
import { findOrCreate } from './utils';

//
// An example which uses low level global methods
//

async function main() {
  //
  // Setup
  //
  const options = Provider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new Provider(connection, adminWallet, options);
  const adminClient = await MangoClient.connect(adminProvider, true);

  // const payer = Keypair.fromSecretKey(
  //   Buffer.from(
  //     JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
  //   ),
  // );
  // console.log(`Payer ${payer.publicKey.toBase58()}`);
  //
  // Find existing or create a new group
  //
  const group: Group = await findOrCreate(
    'group',
    getGroupForAdmin,
    [adminClient, admin.publicKey],
    createGroup,
    [adminClient, admin.publicKey],
  );
  console.log(`Group ${group.publicKey}`);

  //
  // Find existing or register new oracles
  //
  const usdcDevnetMint = new PublicKey(
    '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN',
  );
  const usdcDevnetStubOracle = await findOrCreate<StubOracle>(
    'stubOracle',
    getStubOracleForGroupAndMint,
    [adminClient, group.publicKey, usdcDevnetMint],
    createStubOracle,
    [adminClient, group.publicKey, admin.publicKey, usdcDevnetMint, 1],
  );
  console.log(
    `usdcDevnetStubOracle ${usdcDevnetStubOracle.publicKey.toBase58()}`,
  );
  const btcDevnetMint = new PublicKey(
    '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU',
  );
  const btcDevnetOracle = new PublicKey(
    'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J',
  );

  //
  // Find existing or register new tokens
  //
  // TODO: replace with 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU,
  // see https://developers.circle.com/docs/usdc-on-testnet#usdc-on-solana-testnet
  const btcBank = await findOrCreate<Bank>(
    'bank',
    getBankForGroupAndMint,
    [adminClient, group.publicKey, btcDevnetMint],
    registerToken,
    [
      adminClient,
      group.publicKey,
      admin.publicKey,
      btcDevnetMint,
      btcDevnetOracle,

      0,
    ],
  );
  console.log(`BtcBank ${btcBank.publicKey}`);
  const usdcBank = await findOrCreate<Bank>(
    'bank',
    getBankForGroupAndMint,
    [adminClient, group.publicKey, usdcDevnetMint],
    registerToken,
    [
      adminClient,
      group.publicKey,
      admin.publicKey,
      usdcDevnetMint,
      usdcDevnetStubOracle.publicKey,

      1,
    ],
  );
  console.log(`UsdcBank ${usdcBank.publicKey}`);

  //
  // User operations
  //

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new Provider(connection, userWallet, options);
  const userClient = await MangoClient.connect(userProvider, true);
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  //
  // Create mango account
  //
  const mangoAccount = await findOrCreate<MangoAccount>(
    'mangoAccount',
    getMangoAccountsForGroupAndOwner,
    [userClient, group.publicKey, user.publicKey],
    createMangoAccount,
    [userClient, group.publicKey, user.publicKey, 0],
  );
  console.log(`MangoAccount ${mangoAccount.publicKey}`);

  // close mango account, note: close doesnt settle/withdraw for user atm,
  // only use when you want to free up a mango account address for testing on not-mainnet
  // await closeMangoAccount(userClient, account.publicKey, user.publicKey);
  // accounts = await getMangoAccountsForGroupAndOwner(
  //   userClient,
  //   group.publicKey,
  //   user.publicKey,
  // );
  // if (accounts.length === 0) {
  //   console.log(`Closed account ${account.publicKey}`);
  // }

  //
  // Find existing or register a new serum3 market
  //
  const serumProgramId = new web3.PublicKey(
    'DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY',
  );
  const serumMarketExternalPk = new web3.PublicKey(
    'DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB',
  );
  const serum3Market = await findOrCreate<Serum3Market>(
    'serum3Market',
    getSerum3MarketForBaseAndQuote,
    [adminClient, group.publicKey, btcBank.tokenIndex, usdcBank.tokenIndex],
    serum3RegisterMarket,
    [
      adminClient,
      group.publicKey,
      admin.publicKey,
      serumProgramId,
      serumMarketExternalPk,
      usdcBank.publicKey,
      btcBank.publicKey,
      0,
    ],
  );
  console.log(`Serum3Market ${serum3Market.publicKey}`);

  //
  // Serum3 OO
  //
  if (mangoAccount.serum3[0].marketIndex == 65535) {
    console.log('Creating serum3 open orders account...');
    await serum3CreateOpenOrders(
      userClient,
      group.publicKey,
      mangoAccount.publicKey,
      serum3Market.publicKey,
      serumProgramId,
      serumMarketExternalPk,
      user.publicKey,
    );
  }

  //
  // Deposit & withdraw
  //
  console.log(`Depositing...1000`);
  const btcTokenAccount = await spl.getAssociatedTokenAddress(
    btcDevnetMint,
    user.publicKey,
  );

  // Aggregate all PKs of users active assets, banks, oracles and serum OOs
  const healthRemainingAccounts: PublicKey[] = [];
  {
    const mintInfos = await Promise.all(
      mangoAccount.tokens
        .filter((token) => token.tokenIndex !== 65535)
        .map(async (token) =>
          getMintInfoForTokenIndex(
            userClient,
            group.publicKey,
            token.tokenIndex,
          ),
        ),
    );
    // banks
    healthRemainingAccounts.push(
      ...mintInfos.flatMap((mintinfos) => {
        return mintinfos.flatMap((mintinfo) => {
          return mintinfo.bank;
        });
      }),
    );
    // oracles
    healthRemainingAccounts.push(
      ...mintInfos.flatMap((mintinfos) => {
        return mintinfos.flatMap((mintinfo) => {
          return mintinfo.oracle;
        });
      }),
    );
    // serum OOs
    healthRemainingAccounts.push(
      ...mangoAccount.serum3
        .filter((serum3Account) => serum3Account.marketIndex !== 65535)
        .map((serum3Account) => serum3Account.openOrders),
    );
  }

  await deposit(
    userClient,
    group.publicKey,
    mangoAccount.publicKey,
    btcBank.publicKey,
    btcBank.vault,
    btcTokenAccount,
    user.publicKey,
    healthRemainingAccounts,
    1000,
  );

  console.log(`Witdrawing...500`);
  await withdraw(
    userClient,
    group.publicKey,
    mangoAccount.publicKey,
    btcBank.publicKey,
    btcBank.vault,
    btcTokenAccount,
    user.publicKey,
    healthRemainingAccounts,
    500,
    false,
  );

  // log
  const freshBank = await getBank(userClient, btcBank.publicKey);
  console.log(freshBank.toString());

  let freshAccount = await getMangoAccount(userClient, mangoAccount.publicKey);
  console.log(
    `-  Mango account  ${freshAccount.getNativeDeposit(
      freshBank,
    )} Deposits for bank ${freshBank.tokenIndex}`,
  );

  //
  // Place serum3 order
  //
  console.log('Placing serum3 order...');
  const serum3MarketExternal = await Market.load(
    userClient.program.provider.connection,
    serumMarketExternalPk,
    { commitment: userClient.program.provider.connection.commitment },
    serumProgramId,
  );
  const serum3MarketExternalVaultSigner = await PublicKey.createProgramAddress(
    [
      serumMarketExternalPk.toBuffer(),
      serum3MarketExternal.decoded.vaultSignerNonce.toArrayLike(
        Buffer,
        'le',
        8,
      ),
    ],
    serumProgramId,
  );
  const clientOrderId = Date.now();
  await serum3PlaceOrder(
    userClient,
    group.publicKey,
    mangoAccount.publicKey,
    user.publicKey,
    mangoAccount.serum3[0].openOrders,
    serum3Market.publicKey,
    serumProgramId,
    serumMarketExternalPk,
    serum3MarketExternal.bidsAddress,
    serum3MarketExternal.asksAddress,
    serum3MarketExternal.decoded.eventQueue,
    serum3MarketExternal.decoded.requestQueue,
    serum3MarketExternal.decoded.baseVault,
    serum3MarketExternal.decoded.quoteVault,
    serum3MarketExternalVaultSigner,
    usdcBank.publicKey,
    usdcBank.vault,
    btcBank.publicKey,
    btcBank.vault,
    healthRemainingAccounts,
    Serum3Side.bid,
    40000,
    1,
    1000000,
    Serum3SelfTradeBehavior.decrementTake,
    Serum3OrderType.limit,
    clientOrderId,
    10,
  );

  const ordersForOwner = await serum3MarketExternal.loadOrdersForOwner(
    userClient.program.provider.connection,
    group.publicKey,
  );
  const orderJustPlaced = ordersForOwner.filter(
    (order) => order.clientId?.toNumber() === clientOrderId,
  )[0];
  console.log(`- Serum3 order orderId ${orderJustPlaced.orderId}`);

  process.exit(0);
}

main();
