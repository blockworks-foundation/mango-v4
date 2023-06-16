import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';

//
// example script to close accounts - banks, markets, group etc. which require admin to be the signer
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 200);

const CLUSTER = process.env.CLUSTER || 'mainnet-beta';

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    CLUSTER as Cluster,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
      prioritizationFee: 5,
    },
  );

  const groups = await (async () => {
    return [
      await client.getGroupForCreator(admin.publicKey, Number(GROUP_NUM)),
    ];
  })();
  for (const group of groups) {
    console.log(`Group ${group.publicKey}`);

    let sig;

    // deregister all serum markets
    for (const market of group.serum3MarketsMapByExternal.values()) {
      sig = await client.serum3deregisterMarket(
        group,
        market.serumMarketExternal,
      );
      console.log(
        `Deregistered serum market ${market.name}, sig https://explorer.solana.com/tx/${sig}`,
      );
    }

    // close all perp markets
    for (const market of group.perpMarketsMapByMarketIndex.values()) {
      sig = await client.perpCloseMarket(group, market.perpMarketIndex);
      console.log(
        `Closed perp market ${market.name}, sig https://explorer.solana.com/tx/${sig}`,
      );
    }

    // close all banks
    for (const banks of group.banksMapByMint.values()) {
      sig = await client.tokenDeregister(group, banks[0].mint);
      console.log(
        `Removed token ${banks[0].name}, sig https://explorer.solana.com/tx/${sig}`,
      );
    }

    // close stub oracles
    const stubOracles = await client.getStubOracle(group);
    for (const stubOracle of stubOracles) {
      sig = await client.stubOracleClose(group, stubOracle.publicKey);
      console.log(
        `Closed stub oracle ${stubOracle.publicKey}, sig https://explorer.solana.com/tx/${sig}`,
      );
    }

    // finally, close the group
    sig = await client.groupClose(group);
    console.log(`Closed group, sig https://explorer.solana.com/tx/${sig}`);
  }

  process.exit();
}

main();
