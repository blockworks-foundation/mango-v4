import {WasmAccount, WasmAccounts, compute_health_wasm, Pubkey} from "mango-v4";

import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

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
    false /* Use ids json instead of getProgramAccounts */,
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForAdmin(admin.publicKey, 0);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
    0,
    'my_mango_account',
  );
  let healthAccounts = await client.buildHealthRemainingAccounts(group, mangoAccount);

  let healthAccountInfos = await connection.getMultipleAccountsInfo(healthAccounts);
  let mangoAccountInfo = await connection.getAccountInfo(mangoAccount.publicKey);

  let wasmAccounts = new WasmAccounts;
  for (let i = 0; i < healthAccountInfos.length; ++i) {
    const account = new WasmAccount;
    account.key = new Pubkey(healthAccounts[i].toBase58());
    account.owner = new Pubkey(healthAccountInfos[i].owner.toBase58());
    account.data = healthAccountInfos[i].data;
    wasmAccounts.push(account);
  }

  const wasmMangoAccount = new WasmAccount;
  wasmMangoAccount.key = new Pubkey(mangoAccount.publicKey.toBase58());
  wasmMangoAccount.owner = new Pubkey(mangoAccountInfo.owner.toBase58());
  wasmMangoAccount.data = mangoAccountInfo.data;

  console.log(compute_health_wasm(wasmMangoAccount, wasmAccounts));

  process.exit();
}

main();
