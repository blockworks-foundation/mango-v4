import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { Serum3Side } from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// script which shows how to close a mango account cleanly i.e. close all active positions, withdraw all tokens, etc.
//

// note: either use finalized or expect closing certain things to fail and having to runs scrript multiple times
async function main() {
  const options = AnchorProvider.defaultOptions();

  // note: see note above
  // options.commitment = 'finalized';

  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  // user
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  try {
    // fetch group
    const admin = Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
      ),
    );
    const group = await client.getGroupForAdmin(admin.publicKey, 0);
    console.log(`Found group ${group.publicKey.toBase58()}`);

    // fetch account
    const mangoAccount = (
      await client.getMangoAccountsForOwner(group, user.publicKey)[0]
    )[0];
    console.log(`...found mangoAccount ${mangoAccount.publicKey}`);
    console.log(mangoAccount.toString());

    // close mango account serum3 positions, closing might require cancelling orders and settling
    for (const serum3Account of mangoAccount.serum3Active()) {
      let orders = await client.getSerum3Orders(
        group,
        group.findSerum3Market(serum3Account.marketIndex)!.name,
      );
      for (const order of orders) {
        console.log(
          ` - Order orderId ${order.orderId}, ${order.side}, ${order.price}, ${order.size}`,
        );
        console.log(` - Cancelling order with ${order.orderId}`);
        await client.serum3CancelOrder(
          group,
          mangoAccount,

          'BTC/USDC',
          order.side === 'buy' ? Serum3Side.bid : Serum3Side.ask,
          order.orderId,
        );
      }
      await client.serum3SettleFunds(
        group,
        mangoAccount,
        group.findSerum3Market(serum3Account.marketIndex)!.name,
      );
      await client.serum3CloseOpenOrders(
        group,
        mangoAccount,
        group.findSerum3Market(serum3Account.marketIndex)!.name,
      );
    }

    // we closed a serum account, this changes the health accounts we are passing in for future ixs
    await mangoAccount.reload(client, group);

    // withdraw all tokens
    for (const token of mangoAccount.tokensActive()) {
      let native = token.native(group.findBank(token.tokenIndex)!);

      // to avoid rounding issues
      if (native.toNumber() < 1) {
        continue;
      }
      let nativeFlooredNumber = Math.floor(native.toNumber());
      console.log(
        `withdrawing token ${
          group.findBank(token.tokenIndex)!.name
        } native amount ${nativeFlooredNumber} `,
      );

      await client.tokenWithdrawNative(
        group,
        mangoAccount,
        group.findBank(token.tokenIndex)!.name,
        nativeFlooredNumber,
        false,
        user
      );
    }

    // reload and print current positions
    await mangoAccount.reload(client, group);
    console.log(`...mangoAccount ${mangoAccount.publicKey}`);
    console.log(mangoAccount.toString());

    // close account
    console.log(`Close mango account...`);
    const res = await client.closeMangoAccount(group, mangoAccount);
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

main();
