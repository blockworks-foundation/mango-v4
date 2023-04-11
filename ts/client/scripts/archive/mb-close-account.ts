import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { Serum3Side } from '../../src/accounts/serum3';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

//
// (untested?) script which closes a mango account cleanly, first closes all positions, withdraws all tokens and then closes it
//
async function closeUserAccount(userKeypairFile: string) {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  // user
  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(userKeypairFile, 'utf-8'))),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, 2);
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
      let orders = await mangoAccount.loadSerum3OpenOrdersForMarket(
        client,
        group,
        group.serum3MarketsMapByMarketIndex.get(serum3Account.marketIndex)
          ?.serumMarketExternal!,
      );
      for (const order of orders) {
        console.log(
          ` - Order orderId ${order.orderId}, ${order.side}, ${order.price}, ${order.size}`,
        );
        console.log(` - Cancelling order with ${order.orderId}`);
        await client.serum3CancelOrder(
          group,
          mangoAccount,
          group.serum3MarketsMapByMarketIndex.get(serum3Account.marketIndex)
            ?.serumMarketExternal!,
          order.side === 'buy' ? Serum3Side.bid : Serum3Side.ask,
          order.orderId,
        );
      }

      await client.serum3SettleFunds(
        group,
        mangoAccount,
        group.serum3MarketsMapByMarketIndex.get(serum3Account.marketIndex)
          ?.serumMarketExternal!,
      );
      await client.serum3CloseOpenOrders(
        group,
        mangoAccount,
        group.serum3MarketsMapByMarketIndex.get(serum3Account.marketIndex)
          ?.serumMarketExternal!,
      );
    }

    // we closed a serum account, this changes the health accounts we are passing in for future ixs
    await mangoAccount.reload(client);

    // withdraw all tokens
    for (const token of mangoAccount.tokensActive()) {
      const native = token.balance(
        group.getFirstBankByTokenIndex(token.tokenIndex)!,
      );
      console.log(
        `token native ${native} ${
          group.getFirstBankByTokenIndex(token.tokenIndex)!.name
        }`,
      );
      if (native.toNumber() < 1) {
        continue;
      }

      await client.tokenWithdrawNative(
        group,
        mangoAccount,
        group.getFirstBankByTokenIndex(token.tokenIndex)!.mint,
        new BN(
          token
            .balance(group.getFirstBankByTokenIndex(token.tokenIndex)!)
            .toNumber(),
        ),
        false,
      );
    }
  } catch (error) {
    console.log(error);
  }

  await mangoAccount.reload(client);
  console.log(`...mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  console.log(`Close mango account...`);
  const res = await client.closeMangoAccount(group, mangoAccount);

  process.exit();
}

async function main() {
  // await closeUserAccount(process.env.MB_USER_KEYPAIR!);
  await closeUserAccount(process.env.MB_USER2_KEYPAIR!);
}

main();
