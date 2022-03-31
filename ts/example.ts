import { BN, Provider, Wallet, web3 } from '@project-serum/anchor';
import { Connection, Keypair, SystemProgram } from '@solana/web3.js';
import { MangoClient } from './client';
import os from 'os';
import fs from 'fs';
import { bs58 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { TOKEN_PROGRAM_ID } from '@project-serum/anchor/dist/cjs/utils/token';
import { TokenIndex } from './types';

async function main() {
  const options = Provider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(os.homedir() + '/.config/solana/dev.json', 'utf-8'),
      ),
    ),
  );
  const wallet = new Wallet(admin);

  const payer = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(
          os.homedir() + '/.config/solana/mango-devnet.json',
          'utf-8',
        ),
      ),
    ),
  );

  const provider = new Provider(connection, wallet, options);
  const client = await MangoClient.connect(provider, true);

  //
  // check if group exists, iff not, then create
  //
  let group;
  let gpa = await client.program.account.group.all([
    {
      memcmp: {
        bytes: bs58.encode(admin.publicKey.toBuffer()),
        offset: 8,
      },
    },
  ]);
  if (gpa.length > 0) {
    group = gpa[0];
  } else {
    await client.program.methods
      .createGroup()
      .accounts({
        admin: admin.publicKey,
        payer: admin.publicKey,
        system_program: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    gpa = await client.program.account.group.all([
      {
        memcmp: {
          bytes: bs58.encode(admin.publicKey.toBuffer()),
          offset: 8,
        },
      },
    ]);
    if (gpa.length > 0) {
      group = gpa[0];
    }
  }
  console.log(`Group address: ${group.publicKey.toBase58()}`);

  //
  // check if token is already registered, iff not, then register
  //
  // mngo devnet mint
  const mint = new web3.PublicKey(
    'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC',
  );
  // mngo devnet oracle
  const mngoOracle = new web3.PublicKey(
    '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o',
  );
  let bank;
  gpa = await client.program.account.bank.all([
    {
      memcmp: {
        bytes: bs58.encode(group.publicKey.toBuffer()),
        offset: 8,
      },
    },
    {
      memcmp: {
        bytes: bs58.encode(mint.toBuffer()),
        offset: 40,
      },
    },
  ]);
  if (gpa.length > 0) {
    bank = gpa[0];
  } else {
    await client.program.methods
      .registerToken(0, 0.8, 0.6, 1.2, 1.4, 0.02)
      .accounts({
        group: group.publicKey,
        admin: admin.publicKey,
        mint,
        oracle: mngoOracle,
        payer: payer.publicKey,
        token_program: TOKEN_PROGRAM_ID,
        system_program: SystemProgram.programId,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([admin, payer])
      .rpc();

    gpa = await client.program.account.bank.all([
      {
        memcmp: {
          bytes: bs58.encode(group.publicKey.toBuffer()),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(mint.toBuffer()),
          offset: 40,
        },
      },
    ]);
    bank = gpa[0];
  }
  console.log(`Bank address: ${bank.publicKey.toBase58()}`);
}

main();
