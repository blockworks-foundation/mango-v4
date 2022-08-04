import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { Serum3Side } from '../accounts/serum3';
import { MangoClient } from '../client';

//
// (untested?) script which closes a mango account cleanly, first closes all positions, withdraws all tokens and then closes it
//
async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  // user
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connectForGroupName(
    userProvider,
    'mainnet-beta.microwavedcola' /* Use ids json instead of getProgramAccounts */,
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForAdmin(admin.publicKey);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  // account
  const mangoAccount = (
    await client.getMangoAccountsForOwner(group, user.publicKey)
  )[0];
  console.log(`...found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString(group));

  try {
    // cancel serum3 accounts, closing might require cancelling orders and settling
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
      const native = token.native(group.findBank(token.tokenIndex)!);
      console.log(
        `token native ${native} ${group.findBank(token.tokenIndex)!.name}`,
      );
      if (native.toNumber() < 1) {
        continue;
      }

      await client.tokenWithdrawNative(
        group,
        mangoAccount,
        group.findBank(token.tokenIndex)!.name,
        token.native(group.findBank(token.tokenIndex)!).toNumber(),
        false,
        user,
      );
    }
  } catch (error) {
    console.log(error);
  }

  await mangoAccount.reload(client, group);
  console.log(`...mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  console.log(`Close mango account...`);
  const res = await client.closeMangoAccount(group, mangoAccount);

  process.exit();
}

main();
