import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../../src/accounts/serum3';
import { OpenbookV2Side } from '../../src/accounts/openbookV2';

dotenv.config();

async function addSpotMarket() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const baseMint = new PublicKey('So11111111111111111111111111111111111111112');
  const quoteMint = new PublicKey(
    '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN',
  ); //devnet usdc

  // fetch group
  const groupPk = '7SDejCUPsF3g59GgMsmvxw8dJkkJbT3exoH4RZirwnkM';
  const group = await client.getGroup(new PublicKey(groupPk));
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const account = await client.getMangoAccountForOwner(
    group,
    adminWallet.publicKey,
    0,
    true,
    true,
  );
  if (!account) {
    console.error('no mango account 0');
    return;
  }
  console.log(
    'accountExpand',
    await client.accountExpandV3(
      group,
      account,
      account.tokens.length,
      account.serum3.length,
      account.perps.length,
      account.perpOpenOrders.length,
      0,
      1,
    ),
  );
  console.log([...group.openbookV2ExternalMarketsMap.keys()][0]);
  const marketPk = new PublicKey(
    [...group.openbookV2ExternalMarketsMap.keys()][0],
  );
  console.log(
    'tokenDeposit',
    await client.tokenDeposit(group, account, quoteMint, 1000),
  );
  console.log(
    'placeOrder',
    await client.openbookV2PlaceOrder(
      group,
      account,
      marketPk,
      OpenbookV2Side.bid,
      1,
      1,
      Serum3SelfTradeBehavior.decrementTake,
      Serum3OrderType.limit,
      420,
      32,
    ),
  );

  process.exit();
}

async function main() {
  await addSpotMarket();
}

main();
