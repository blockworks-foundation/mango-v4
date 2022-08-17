import { AnchorProvider } from '@project-serum/anchor';
import {
  AddressLookupTableProgram,
  Connection,
  Keypair,
  Transaction,
} from '@solana/web3.js';
import fs from 'fs';

async function main() {
  let res;

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );

  ///

  const createIx = AddressLookupTableProgram.createLookupTable({
    authority: admin.publicKey,
    payer: admin.publicKey,
    recentSlot: await connection.getSlot(),
  });

  const createTx = new Transaction();
  createTx.add(createIx[0]);
  createTx.feePayer = admin.publicKey;

  try {
    res = await connection.sendTransaction(createTx, [admin]);
    console.log(`https://explorer.solana.com/tx/${res}?cluster=devnet`);
  } catch (error) {
    console.log(res);
  }

  const alt = createIx[1];

  ///

  const deactivateIx = AddressLookupTableProgram.deactivateLookupTableale({
    lookupTable: alt,
    authority: admin.publicKey,
  });

  const deactivateTx = new Transaction();
  deactivateTx.add(deactivateIx);

  try {
    res = await connection.sendTransaction(deactivateTx, [admin]);
    console.log(`https://explorer.solana.com/tx/${res}?cluster=devnet`);
  } catch (error) {
    console.log(res);
  }

  ///

  const closeIx = AddressLookupTableProgram.closeLookupTable({
    lookupTable: alt,
    authority: admin.publicKey,
    recipient: admin.publicKey,
  });

  const closeTx = new Transaction();
  closeTx.add(closeIx);

  try {
    res = await connection.sendTransaction(closeTx, [admin]);
    console.log(`https://explorer.solana.com/tx/${res}?cluster=devnet`);
  } catch (error) {
    console.log(res);
  }

  process.exit();
}

main();
