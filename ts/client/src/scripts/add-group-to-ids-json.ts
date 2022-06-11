import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import idsJson from '../../ids.json';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { Id } from '../ids';

function replacer(key, value) {
  if (value instanceof Map) {
    return Object.fromEntries(value);
  } else {
    return value;
  }
}

async function main() {
  // build client and fetch group for admin
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
    false,
  );
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
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
    new Map(banks.map((tuple) => [tuple.name, tuple.publicKey])),
    new Map(
      stubOracles.map((tuple) => [
        banksMapByMint.get(tuple.mint.toBase58())!.name,
        tuple.publicKey,
      ]),
    ),
    new Map(
      mintInfos.map((tuple) => [
        banksMapByMint.get(tuple.mint.toBase58())!.name,
        tuple.publicKey,
      ]),
    ),
    new Map(serum3Markets.map((tuple) => [tuple.name, tuple.publicKey])),
    new Map(
      serum3Markets.map((tuple) => [tuple.name, tuple.serumMarketExternal]),
    ),
    new Map(perpMarkets.map((tuple) => [tuple.name, tuple.publicKey])),
  );

  // adds ids for group in existing ids.json
  idsJson['devnet'][MANGO_V4_ID['devnet'].toBase58()] = {};
  idsJson['devnet'][MANGO_V4_ID['devnet'].toBase58()][
    group.publicKey.toBase58()
  ] = toDump;

  // dump
  const file = `${process.cwd()}/ts/client/ids.json`;
  await fs.writeFileSync(file, JSON.stringify(idsJson, replacer, 2));

  process.exit();
}
main();
