import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

//
// This script tries to withdraw all positive balances for all accounts
// by MANGO_MAINNET_PAYER_KEYPAIR in the group.
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 1);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(admin);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  let accounts = await client.getMangoAccountsForOwner(group, admin.publicKey);
  for (let account of accounts) {
    console.log(`settling borrows on account: ${account}`);

    // first, settle all borrows
    for (let token of account.tokensActive()) {
      const bank = group.findBank(token.tokenIndex);
      const amount = token.native(bank).toNumber();
      if (amount < 0) {
        try {
          await client.tokenDepositNative(group, account, bank.name, amount);
          await account.reload(client, group);
        } catch (error) {
          console.log(
            `failed to deposit ${bank.name} into ${account.publicKey}: ${error}`,
          );
        }
      }
    }
  }

  accounts = await client.getMangoAccountsForOwner(group, admin.publicKey);
  for (let account of accounts) {
    console.log(`withdrawing deposits of account: ${account}`);

    // withdraw all funds
    for (let token of account.tokensActive()) {
      const bank = group.findBank(token.tokenIndex);
      const amount = token.native(bank).toNumber();
      if (amount > 0) {
        try {
          const allowBorrow = true; // TODO: set this to false once the withdraw amount ___<___ nativePosition bug is fixed
          await client.tokenWithdrawNative(
            group,
            account,
            bank.name,
            amount,
            allowBorrow,
          );
          await account.reload(client, group);
        } catch (error) {
          console.log(
            `failed to withdraw ${bank.name} from ${account.publicKey}: ${error}`,
          );
        }
      }
    }

    // close account
    try {
      console.log(`closing account: ${account}`);
      await client.closeMangoAccount(group, account);
    } catch (error) {
      console.log(`failed to close ${account.publicKey}: ${error}`);
    }
  }

  process.exit();
}

main();
