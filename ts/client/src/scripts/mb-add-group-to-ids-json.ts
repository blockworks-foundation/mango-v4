import { AnchorProvider, Wallet } from '@project-serum/anchor';

import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import idsJson from '../../ids.json';
import { MangoClient } from '../client';
import { MANGO_V4_ID, SERUM3_PROGRAM_ID } from '../constants';
import { Id } from '../ids';

function replacer(key, value) {
  if (value instanceof Map) {
    return Object.fromEntries(value);
  } else {
    return value;
  }
}

async function main() {
  const groupName = 'mainnet-beta.microwavedcola';
  const cluster = 'mainnet-beta';

  // build client and fetch group for admin
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    cluster,
    MANGO_V4_ID[cluster],
  );
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const group = await client.getGroupForAdmin(admin.publicKey, 0);

  // collect mappings &
  // collect pubkeys
  const banks = await client.getBanksForGroup(group);
  const banksMapByMint = new Map(
    banks.map((tuple) => [tuple.mint.toBase58(), tuple]),
  );
  const stubOracles = await client.getStubOracle(group);
  const mintInfos = await client.getMintInfosForGroup(group);
  const serum3Markets = await client.serum3GetMarkets(group);
  const perpMarkets = await client.perpGetMarkets(group);

  // build ids
  const toDump = new Id(
    cluster,
    groupName,
    group.publicKey.toBase58(),
    SERUM3_PROGRAM_ID[cluster].toBase58(),
    MANGO_V4_ID[cluster].toBase58(),
    banks.map((tuple) => ({
      name: tuple.name,
      publicKey: tuple.publicKey.toBase58(),
    })),
    stubOracles.map((tuple) => ({
      name: banksMapByMint.get(tuple.mint.toBase58())!.name,
      publicKey: tuple.publicKey.toBase58(),
    })),
    mintInfos.map((tuple) => ({
      name: banksMapByMint.get(tuple.mint.toBase58())!.name,
      publicKey: tuple.publicKey.toBase58(),
    })),
    serum3Markets.map((tuple) => ({
      name: tuple.name,
      publicKey: tuple.publicKey.toBase58(),
      marketExternal: tuple.serumMarketExternal.toBase58(),
    })),
    perpMarkets.map((tuple) => ({
      name: tuple.name,
      publicKey: tuple.publicKey.toBase58(),
    })),
  );

  console.log(toDump);

  // adds ids for group in existing ids.json
  const existingGroup = idsJson.groups.find((group) => group.name == groupName);
  if (existingGroup) {
    console.log('Updating existing group with latest state...');
  } else {
    console.log('Group does not exist yet...');
  }
  idsJson.groups = idsJson.groups.filter((group) => group.name !== groupName);
  idsJson.groups.push(toDump);

  // dump
  const file = `${process.cwd()}/ts/client/ids.json`;
  await fs.writeFileSync(file, JSON.stringify(idsJson, replacer, 2));

  process.exit();
}
main();
