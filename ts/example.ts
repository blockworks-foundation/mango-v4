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

  const privateKeyPath = os.homedir() + '/.config/solana/dev.json';
  const owner = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(privateKeyPath, 'utf-8'))),
  );
  const wallet = new Wallet(owner);

  const provider = new Provider(connection, wallet, options);
  const client = await MangoClient.connect(provider, true);

  let group;
  let gpa = await client.program.account.group.all([
    {
      memcmp: {
        bytes: bs58.encode(owner.publicKey.toBuffer()),
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
        admin: owner.publicKey,
        payer: owner.publicKey,
        system_program: SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    gpa = await client.program.account.group.all([
      {
        memcmp: {
          bytes: bs58.encode(owner.publicKey.toBuffer()),
          offset: 8,
        },
      },
    ]);
    if (gpa.length > 0) {
      group = gpa[0];
    }
  }
  console.log(`Group address: ${group.publicKey.toBase58()}`);

  // mngo devnet mint
  const mint = new web3.PublicKey(
    'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC',
  );
  // mngo devnet oracle
  const mngoOracle = new web3.PublicKey(
    '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o',
  );
  // some random address atm
  const address_lookup_table = new web3.PublicKey(
    '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o',
  );
  const address_lookup_table_program = new web3.PublicKey(
    'AddressLookupTab1e1111111111111111111111111',
  );
  await client.program.methods
    .registerToken(
      TokenIndex.fromValue(new BN(0)) as any,
      0.8,
      0.6,
      1.2,
      1.4,
      0.02,
    )
    .accounts({
      group: group.publicKey,
      admin: owner.publicKey,
      mint,
      oracle: mngoOracle,
      address_lookup_table,
      payer: owner.publicKey,
      token_program: TOKEN_PROGRAM_ID,
      system_program: SystemProgram.programId,
      address_lookup_table_program,
      rent: web3.SYSVAR_RENT_PUBKEY,
    })
    .signers([owner])
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
  console.log(gpa);
  // const bank = gpa[0];
  // console.log(bank.publicKey.toBase58());
}

main();
