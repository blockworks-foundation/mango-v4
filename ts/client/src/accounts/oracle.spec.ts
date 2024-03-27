import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import {
  USDC_MINT_MAINNET,
  deriveFallbackOracleQuoteKey,
  isOrcaOracle,
  isRaydiumOracle,
} from './oracle';
import { MangoClient } from '../client';
import { MANGO_V4_ID, MANGO_V4_MAIN_GROUP } from '../constants';
import { AnchorProvider, Provider, Wallet } from '@coral-xyz/anchor';
import * as fs from 'fs';
import { Group } from './group';

function getProvider(connection: Connection): Provider {
  const secretKey = JSON.parse(
    fs.readFileSync(process.env.KEYPAIR_PATH as string, 'utf-8'),
  );
  const kp = Keypair.fromSecretKey(Uint8Array.from(secretKey));
  const wallet = new Wallet(kp);
  const provider = new AnchorProvider(connection, wallet, {});
  return provider;
}

describe.only('Oracle', () => {
  const connection = new Connection('https://api.mainnet-beta.solana.com/');
  const CLUSTER = 'mainnet-beta';

  const Orca_SOL_USDC_Whirlpool = new PublicKey(
    '83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d',
  );
  const Raydium_SOL_USDC_Whirlpool = new PublicKey(
    'Ds33rQ1d4AXwxqyeXX6Pc3G4pFNr6iWb3dd8YfBBQMPr',
  );

  it('can decode Orca CLMM oracles', async () => {
    const accInfo = await connection.getAccountInfo(Orca_SOL_USDC_Whirlpool);
    expect(accInfo).not.to.be.null;
    expect(isOrcaOracle(accInfo!)).to.be.true;

    const other = await connection.getAccountInfo(Raydium_SOL_USDC_Whirlpool);
    expect(isOrcaOracle(other!)).to.be.false;

    const quoteKey = deriveFallbackOracleQuoteKey(accInfo!);
    expect(quoteKey.equals(USDC_MINT_MAINNET)).to.be.true;
  });

  it('can decode Raydium CLMM oracles', async () => {
    const accInfo = await connection.getAccountInfo(Raydium_SOL_USDC_Whirlpool);
    expect(accInfo).not.to.be.null;
    expect(isRaydiumOracle(accInfo!)).to.be.true;

    const other = await connection.getAccountInfo(Orca_SOL_USDC_Whirlpool);
    expect(isRaydiumOracle(other!)).to.be.false;

    const quoteKey = deriveFallbackOracleQuoteKey(accInfo!);
    expect(quoteKey.equals(USDC_MINT_MAINNET)).to.be.true;
  });

  it.skip('can generate fixed fallback oracles', async () => {
    const provider = getProvider(connection);
    const client = MangoClient.connect(
      provider,
      CLUSTER,
      MANGO_V4_ID[CLUSTER],
      {
        fallbackOracleConfig: [
          new PublicKey('H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'),
        ],
      },
    ); // SOL

    const groupAccount = await client.program.account.group.fetch(
      MANGO_V4_MAIN_GROUP,
    );
    const GROUP = Group.from(MANGO_V4_MAIN_GROUP, groupAccount);
    await GROUP.reloadBanks(client);
    const fbs = await client.deriveFallbackOracleContexts(GROUP);
    expect(fbs.size).to.equal(1);
  });

  it.skip('can generate all fallback oracles', async () => {
    const provider = getProvider(connection);
    const client = MangoClient.connect(
      provider,
      CLUSTER,
      MANGO_V4_ID[CLUSTER],
      { fallbackOracleConfig: 'all' },
    );

    const groupAccount = await client.program.account.group.fetch(
      MANGO_V4_MAIN_GROUP,
    );
    const GROUP = Group.from(MANGO_V4_MAIN_GROUP, groupAccount);
    await GROUP.reloadBanks(client);
    const fbs = await client.deriveFallbackOracleContexts(GROUP);
    expect(fbs.size).to.be.greaterThan(1);
  });

  it.skip('can generate dynamic fallback oracles', async () => {
    const provider = getProvider(connection);
    const client = MangoClient.connect(
      provider,
      CLUSTER,
      MANGO_V4_ID[CLUSTER],
      { fallbackOracleConfig: 'dynamic' },
    );

    const groupAccount = await client.program.account.group.fetch(
      MANGO_V4_MAIN_GROUP,
    );
    const GROUP = Group.from(MANGO_V4_MAIN_GROUP, groupAccount);
    await GROUP.reloadBanks(client);
    const fbs = await client.deriveFallbackOracleContexts(GROUP);
    console.log(fbs.size);
  });
});
