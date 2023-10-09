import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoAccount } from '../../src/accounts/mangoAccount';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

//
// An example for users based on high level api i.e. the client
// Create
// process.env.USER_KEYPAIR - mango account owner keypair path
// process.env.ADMIN_KEYPAIR - group admin keypair path (useful for automatically finding the group)
//
// This script deposits some tokens, places some serum orders, cancels them, places some perp orders
//

const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['ORCA', 'orcarKHSqC5CDDsGbho8GKvwExejWHxTqGzXgcewB9L'],
  ['MNGO', 'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC'],
  ['ETH', 'Cu84KB3tDL6SbFgToHMLYVDJJXdJjenNzSKikeAvzmkA'],
  ['SRM', 'AvtB6w9xboLwA145E221vhof5TddhqsChYcx7Fy3xVMH'],
]);
const DEVNET_ORACLES = new Map([
  ['BTC', 'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J'],
  ['SOL', 'J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix'],
  ['ORCA', 'A1WttWF7X3Rg6ZRpB2YQUFHCRh1kiXV8sKKLV3S9neJV'],
  ['MNGO', '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o'],
  ['ETH', 'EdVCmQ9FSPcVe5YySXDPCRmc8aDQLKJ9xvYBMZPie1Vw'],
  ['SRM', '992moaMQKs32GKZ9dxi8keyM2bUmbrwBZpK4p2K6X5Vs'],
]);
export const DEVNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', new PublicKey('DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB')],
  ['SOL/USDC', new PublicKey('5xWpt56U1NCuHoAEtpLeUrQcxDkEpNfScjfLFaRzLPgR')],
]);

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER2_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
    {
      idsSource: 'get-program-accounts',
    },
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = (await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  )) as MangoAccount;
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);

  // eslint-disable-next-line no-constant-condition
  if (true) {
    // deposit and withdraw

    try {
      console.log(`...depositing`);
      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('USDC')!),
        1000,
      );
      await mangoAccount.reload(client);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('MNGO')!),
        100,
      );
      await mangoAccount.reload(client);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('ETH')!),
        500,
      );
      await mangoAccount.reload(client);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('SRM')!),
        500,
      );
      await mangoAccount.reload(client);

      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS.get('BTC')!),
        1,
      );
      await mangoAccount.reload(client);

      console.log(mangoAccount.toString(group));
    } catch (error) {
      console.log(error);
    }
  }

  // expand account
  if (
    mangoAccount.tokens.length < 16 ||
    mangoAccount.serum3.length < 8 ||
    mangoAccount.perps.length < 8 ||
    mangoAccount.perpOpenOrders.length < 8
  ) {
    console.log(
      `...expanding mango account to max 16 token positions, 8 serum3, 8 perp position and 8 perp oo slots, previous (tokens ${mangoAccount.tokens.length}, serum3 ${mangoAccount.serum3.length}, perps ${mangoAccount.perps.length}, perps oo ${mangoAccount.perpOpenOrders.length})`,
    );
    const sig = await client.expandMangoAccount(
      group,
      mangoAccount,
      16,
      8,
      8,
      8,
    );
    console.log(`sig https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    await mangoAccount.reload(client);
  }

  process.exit();
}

main();
