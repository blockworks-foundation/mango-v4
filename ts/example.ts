import { Provider, Wallet, web3, BN } from '@project-serum/anchor';
import { bs58 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { TOKEN_PROGRAM_ID } from '@project-serum/anchor/dist/cjs/utils/token';
import { Connection, Keypair, SystemProgram } from '@solana/web3.js';
import fs from 'fs';
import os from 'os';
import { MangoClient } from './client';

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
  // console.log(group);

  //
  // check if token is already registered, iff not, then register
  //
  // mngo devnet mint
  const mngoDevnetMint = new web3.PublicKey(
    'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC',
  );
  const btcDevnetMint = new web3.PublicKey(
    '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU',
  );
  // mngo devnet oracle
  const mngoDevnetOracle = new web3.PublicKey(
    '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o',
  );
  const btcDevnetOracle = new web3.PublicKey(
    'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J',
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
        bytes: bs58.encode(btcDevnetMint.toBuffer()),
        offset: 40,
      },
    },
  ]);
  if (gpa.length > 0) {
    bank = gpa[0];
  } else {
    await client.program.methods
      .registerToken(1, 0.8, 0.6, 1.2, 1.4, 0.02)
      .accounts({
        group: group.publicKey,
        admin: admin.publicKey,
        mint: btcDevnetMint,
        oracle: btcDevnetOracle,
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
          bytes: bs58.encode(btcDevnetMint.toBuffer()),
          offset: 40,
        },
      },
    ]);
    bank = gpa[0];
  }
  console.log(`Bank address: ${bank.publicKey.toBase58()}`);
  // console.log(bank.account.vault);

  //
  // mango account
  //

  let mangoAccount;
  // gpa = await client.program.account.mangoAccount.all([
  //   {
  //     memcmp: {
  //       bytes: bs58.encode(group.publicKey.toBuffer()),
  //       offset: 8,
  //     },
  //   },
  //   {
  //     memcmp: {
  //       bytes: bs58.encode(admin.publicKey.toBuffer()),
  //       offset: 40,
  //     },
  //   },
  // ]);
  // if (gpa.length > 0) {
  //   mangoAccount = gpa[0];
  // } else {

  // await client.program.methods
  //   .createAccount(11)
  //   .accounts({
  //     group: group.publicKey,
  //     owner: admin.publicKey,
  //     payer: payer.publicKey,
  //     system_program: SystemProgram.programId,
  //   })
  //   .signers([admin, payer])
  //   .rpc();

  //   gpa = await client.program.account.mangoAccount.all([
  //     {
  //       memcmp: {
  //         bytes: bs58.encode(group.publicKey.toBuffer()),
  //         offset: 8,
  //       },
  //     },
  //     {
  //       memcmp: {
  //         bytes: bs58.encode(admin.publicKey.toBuffer()),
  //         offset: 40,
  //       },
  //     },
  //   ]);
  //   mangoAccount = gpa[0];
  // }

  let mangoAccountPk = new web3.PublicKey(
    'CtdYxnaWZPgD5BuchHmo2fKecJbVberhqBRbrspsw9gc',
  );
  mangoAccount = await client.program.account.mangoAccount.fetch(
    mangoAccountPk,
  );
  console.log(`Mango account address: ${mangoAccountPk.toBase58()}`);
  // console.log(mangoAccount);

  //
  // deposit
  //
  let mngoTokenAccount = new web3.PublicKey(
    'EnaCw8ZooD1sFbqz2J3w8XAZmtp5sAvVx1RxKM2pMVLh',
  );
  let btcTokenAccount = new web3.PublicKey(
    'DS2vYFVtQbbJDowCG4NEM9KGQ8TJpxKo5efBQj96eCPS',
  );

  await client.program.methods
    .deposit(new BN(0))
    .accounts({
      group: group.publicKey,
      account: mangoAccountPk,
      bank: bank.publicKey,
      vault: bank.account.vault,
      tokenAccount: btcTokenAccount,
      tokenAuthority: admin.publicKey,
      token_program: TOKEN_PROGRAM_ID,
    })
    .remainingAccounts([
      { pubkey: bank.publicKey, isWritable: false, isSigner: false },
      { pubkey: btcDevnetOracle, isWritable: false, isSigner: false },
    ])
    .signers([admin])
    .rpc();

  mangoAccount = await client.program.account.mangoAccount.fetch(
    mangoAccountPk,
  );
  for (const tokenAccount of mangoAccount.tokenAccountMap.values) {
    if (tokenAccount.tokenIndex !== 65535) {
      console.log(
        `${
          tokenAccount.tokenIndex
        } - ${tokenAccount.indexedValue.val.toNumber()}`,
      );
    }
  }

  await client.program.methods
    .withdraw(new BN(1000), false)
    .accounts({
      group: group.publicKey,
      account: mangoAccountPk,
      owner: admin.publicKey,
      bank: bank.publicKey,
      vault: bank.account.vault,
      tokenAccount: btcTokenAccount,
      token_program: TOKEN_PROGRAM_ID,
    })
    .remainingAccounts([
      { pubkey: bank.publicKey, isWritable: false, isSigner: false },
      { pubkey: btcDevnetOracle, isWritable: false, isSigner: false },
    ])
    .signers([admin])
    .rpc();
  mangoAccount = await client.program.account.mangoAccount.fetch(
    mangoAccountPk,
  );
  for (const tokenAccount of mangoAccount.tokenAccountMap.values) {
    if (tokenAccount.tokenIndex !== 65535) {
      console.log(
        `${
          tokenAccount.tokenIndex
        } - ${tokenAccount.indexedValue.val.toNumber()}`,
      );
    }
  }
}

main();
