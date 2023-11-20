import { AnchorProvider } from '@coral-xyz/anchor';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
  AddressLookupTableProgram,
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  SystemProgram,
} from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../src/accounts/group';
import { MangoClient } from '../src/client';
import { MANGO_V4_MAIN_GROUP, OPENBOOK_PROGRAM_ID } from '../src/constants';
import { buildVersionedTx } from '../src/utils';
const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, DRY_RUN } = process.env;

// eslint-disable-next-line no-inner-declarations
async function extendTable(
  client: MangoClient,
  group: Group,
  payer: Keypair,
  nick: string,
  altAddress: PublicKey,
  addresses: PublicKey[],
): Promise<void> {
  await group.reloadAll(client);
  const alt = await client.program.provider.connection.getAddressLookupTable(
    altAddress,
  );

  addresses = addresses.filter(
    (newAddress) =>
      alt.value?.state.addresses &&
      alt.value?.state.addresses.findIndex((addressInALt) =>
        addressInALt.equals(newAddress),
      ) === -1,
  );
  if (addresses.length === 0) {
    return;
  }
  const extendIx = AddressLookupTableProgram.extendLookupTable({
    lookupTable: group.addressLookupTables[0],
    payer: payer.publicKey,
    authority: payer.publicKey,
    addresses,
  });

  if (DRY_RUN) {
    console.log(
      `Extending ${altAddress} with ${nick} ${
        addresses.length
      } addresses - ${addresses.join(', ')}`,
    );
    return;
  }
  const extendTx = await buildVersionedTx(
    client.program.provider as AnchorProvider,
    [extendIx],
  );
  //   const sig = await client.program.provider.connection.sendTransaction(
  //     extendTx,
  //   );
  //   console.log(`https://explorer.solana.com/tx/${sig}`);
}

async function run(): Promise<void> {
  const client = await MangoClient.connectDefault(MB_CLUSTER_URL!);
  const group = await client.getGroup(MANGO_V4_MAIN_GROUP);

  const payer = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );

  //
  // Table 0 - liquidation relevant accounts
  //
  //   const altAddress0 = group.addressLookupTablesList[0].key;
  const altAddress0 = new PublicKey(
    'AgCBUZ6UMWqPLftTxeAqpQxtrfiCyL2HgRfmmM6QTfCj',
  );
  // group and insurance vault
  await extendTable(client, group, payer, 'group', altAddress0, [
    group.publicKey,
    group.insuranceVault,
  ]);
  // Banks + vaults + oracles
  // Split into 3 ixs since we end up with RangeError: encoding overruns Uint8Array otherwise
  await extendTable(
    client,
    group,
    payer,
    'token banks',
    altAddress0,
    Array.from(group.banksMapByMint.values())
      .flat()
      .map((bank) => bank.publicKey),
  );
  await extendTable(
    client,
    group,
    payer,
    'token bank oracles',
    altAddress0,
    Array.from(group.banksMapByMint.values())
      .flat()
      .map((bank) => bank.oracle),
  );
  await extendTable(
    client,
    group,
    payer,
    'token bank vaults',
    altAddress0,
    Array.from(group.banksMapByMint.values())
      .flat()
      .map((bank) => bank.vault),
  );
  // Perps + oracles
  await extendTable(
    client,
    group,
    payer,
    'perp markets and perp oracles',
    altAddress0,
    Array.from(group.perpMarketsMapByMarketIndex.values())
      .flat()
      .map((perpMarket) => [perpMarket.publicKey, perpMarket.oracle])
      .flat(),
  );
  // Well known addressess
  await extendTable(client, group, payer, 'well known addresses', altAddress0, [
    // Solana specific
    SystemProgram.programId,
    SYSVAR_RENT_PUBKEY,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    NATIVE_MINT,
    SYSVAR_INSTRUCTIONS_PUBKEY,
    ComputeBudgetProgram.programId,
    // Misc.
    OPENBOOK_PROGRAM_ID['mainnet-beta'],
  ]);

  //
  // Table 1 - everything else
  //
  //   const altAddress1 = group.addressLookupTablesList[1].key;
  const altAddress1 = new PublicKey(
    'FGZCgVhVGqzfWnmJFP9Hx4BvGvnFApEp1dM2whzXvg1Z',
  );
  // bank mints
  await extendTable(
    client,
    group,
    payer,
    'token mints',
    altAddress1,
    Array.from(group.banksMapByMint.values())
      .flat()
      .map((bank) => [bank.mint])
      .flat(),
  );
  // bank mint infos
  await extendTable(
    client,
    group,
    payer,
    'mint infos',
    altAddress1,
    Array.from(group.mintInfosMapByMint.values())
      .flat()
      .map((mintInto) => [mintInto.publicKey])
      .flat(),
  );
  // serum3
  // Split into 4 ixs since we end up with RangeError: encoding overruns Uint8Array otherwise
  await extendTable(
    client,
    group,
    payer,
    'serum3 markets',
    altAddress1,
    Array.from(group.serum3MarketsMapByMarketIndex.values())
      .flat()
      .map((serum3Market) => serum3Market.publicKey),
  );
  await extendTable(
    client,
    group,
    payer,
    'serum3 external markets',
    altAddress1,
    Array.from(group.serum3ExternalMarketsMap.values())
      .flat()
      .map((serum3ExternalMarket) => serum3ExternalMarket.publicKey),
  );
  await extendTable(
    client,
    group,
    payer,
    'serum3 external markets bids',
    altAddress1,
    Array.from(group.serum3ExternalMarketsMap.values())
      .flat()
      .map((serum3ExternalMarket) => serum3ExternalMarket.bidsAddress),
  );
  await extendTable(
    client,
    group,
    payer,
    'serum3 external markets asks',
    altAddress1,
    Array.from(group.serum3ExternalMarketsMap.values())
      .flat()
      .map((serum3ExternalMarket) => serum3ExternalMarket.asksAddress),
  );
}

run();
