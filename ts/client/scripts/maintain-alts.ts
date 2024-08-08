import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
  AddressLookupTableProgram,
  Cluster,
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
} from '@solana/web3.js';
import {
  fetchAllLutKeys,
  ON_DEMAND_MAINNET_QUEUE,
  PullFeed,
  Queue,
  SB_ON_DEMAND_PID,
} from '@switchboard-xyz/on-demand';
import fs from 'fs';
import chunk from 'lodash/chunk';
import { Program as Anchor30Program } from 'switchboard-anchor';
import { Group } from '../src/accounts/group';
import { MangoClient } from '../src/client';
import {
  MANGO_V4_ID,
  MANGO_V4_MAIN_GROUP,
  OPENBOOK_PROGRAM_ID,
  SBOD_ORACLE_LUTS,
} from '../src/constants';
import { buildVersionedTx } from '../src/utils';
import { getOraclesForMangoGroup } from './sb-on-demand-crank-utils';

const { MB_CLUSTER_URL, MB_PAYER3_KEYPAIR, DRY_RUN } = process.env;
const CLUSTER: Cluster = (process.env.CLUSTER as Cluster) || 'mainnet-beta';

async function buildSbOnDemandAccountsForAlts(
  connection: Connection,
  group: Group,
): Promise<PublicKey[]> {
  const userProvider = new AnchorProvider(
    connection,
    new Wallet(Keypair.generate()),
    AnchorProvider.defaultOptions(),
  );
  const idl = await Anchor30Program.fetchIdl(SB_ON_DEMAND_PID, userProvider);
  const sbOnDemandProgram = new Anchor30Program(idl!, userProvider);

  // all sbod oracles on mango group
  const oracles = getOraclesForMangoGroup(group);
  const ais = (
    await Promise.all(
      chunk(
        oracles.map((item) => item.oraclePk),
        50,
        false,
      ).map(async (chunk) => await connection.getMultipleAccountsInfo(chunk)),
    )
  ).flat();
  const sbodOracles = oracles
    .map((o, i) => {
      return { oracle: o, ai: ais[i] };
    })
    .filter((item) => item.ai?.owner.equals(SB_ON_DEMAND_PID));

  return await fetchAllLutKeys(
    new Queue(sbOnDemandProgram, new PublicKey(ON_DEMAND_MAINNET_QUEUE)),
    sbodOracles.map((oracle) => {
      return new PullFeed(sbOnDemandProgram, oracle.oracle.oraclePk);
    }),
  );
}

// eslint-disable-next-line no-inner-declarations
async function extendTable(
  client: MangoClient,
  group: Group,
  payer: Keypair,
  nick: string,
  altAddresses: PublicKey[],
  addressesToAdd: PublicKey[],
): Promise<void> {
  await group.reloadAll(client);

  let addressesAlreadyIndexed: PublicKey[] = [];
  for (const altAddr of altAddresses) {
    const alt =
      await client.program.provider.connection.getAddressLookupTable(altAddr);
    if (alt.value?.state.addresses) {
      addressesAlreadyIndexed = addressesAlreadyIndexed.concat(
        alt.value?.state.addresses,
      );
    }
  }

  addressesToAdd = addressesToAdd.filter(
    (newAddress) =>
      addressesAlreadyIndexed.findIndex((addressInAlt) =>
        addressInAlt.equals(newAddress),
      ) === -1,
  );
  if (addressesToAdd.length === 0) {
    return;
  }

  let altIndex = 0;
  for (const chunk_ of chunk(addressesToAdd, 20)) {
    let alt;
    while (altIndex < altAddresses.length) {
      alt = await client.program.provider.connection.getAddressLookupTable(
        altAddresses[altIndex],
      );
      if (alt.value?.state.addresses.length < 234) {
        break;
      } else {
        if (altIndex == altAddresses.length - 1) {
          console.log(
            `...need to create a new alt, all existing ones are full, ${nick}`,
          );
          process.exit(-1);
        }
        console.log(
          `...using a new alt ${altAddresses[altIndex + 1]}, ${
            altAddresses[altIndex]
          } is almost full`,
        );
      }
      altIndex++;
    }

    console.log(
      `Extending ${altAddresses[altIndex]} with ${nick} ${
        chunk_.length
      } addresses - ${chunk_.join(', ')}`,
    );

    const extendIx = AddressLookupTableProgram.extendLookupTable({
      lookupTable: alt.value!.key,
      payer: payer.publicKey,
      authority: payer.publicKey,
      addresses: chunk_,
    });
    const extendTx = await buildVersionedTx(
      client.program.provider as AnchorProvider,
      [extendIx],
    );

    if (DRY_RUN) {
      continue;
    }
    const sig = await client.program.provider.connection.sendTransaction(
      extendTx,
      { skipPreflight: true },
    );
    console.log(`https://explorer.solana.com/tx/${sig}`);
  }
}

async function createANewAlt() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);
  const payer = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER3_KEYPAIR!, 'utf-8'))),
  );
  const payerWallet = new Wallet(payer);
  const userProvider = new AnchorProvider(connection, payerWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'api',
    },
  );

  const createIx = AddressLookupTableProgram.createLookupTable({
    authority: payer.publicKey,
    payer: payer.publicKey,
    recentSlot: await connection.getSlot('finalized'),
  });
  const createTx = await buildVersionedTx(
    client.program.provider as AnchorProvider,
    [createIx[0]],
  );
  const sig = await connection.sendTransaction(createTx, {
    skipPreflight: true,
  });
  console.log(
    `...created ALT ${createIx[1]} https://explorer.solana.com/tx/${sig}`,
  );
}

async function populateExistingAltsWithMangoGroupAccounts(): Promise<void> {
  try {
    const options = AnchorProvider.defaultOptions();
    const connection = new Connection(MB_CLUSTER_URL!, options);
    const payer = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER3_KEYPAIR!, 'utf-8'))),
    );
    const payerWallet = new Wallet(payer);
    const userProvider = new AnchorProvider(connection, payerWallet, options);
    const client = await MangoClient.connect(
      userProvider,
      CLUSTER,
      MANGO_V4_ID[CLUSTER],
      {
        idsSource: 'api',
      },
    );
    const group = await client.getGroup(MANGO_V4_MAIN_GROUP);

    //
    // Table 0 - liquidation relevant accounts
    //
    //   const altAddress0 = group.addressLookupTablesList[0].key;
    const altAddress0 = new PublicKey(
      'AgCBUZ6UMWqPLftTxeAqpQxtrfiCyL2HgRfmmM6QTfCj',
    );
    const altAddress11 = new PublicKey(
      '5iCJfe8RqQ3DFeP8uHXYe8Q6hFPYVh8PfBX7rU9ydC99',
    );
    // group and insurance vault
    await extendTable(
      client,
      group,
      payer,
      'group',
      [altAddress0, altAddress11],
      [group.publicKey, group.insuranceVault],
    );
    // Banks + vaults + oracles
    // Split into 3 ixs since we end up with RangeError: encoding overruns Uint8Array otherwise
    await extendTable(
      client,
      group,
      payer,
      'token banks',
      [altAddress0, altAddress11],
      Array.from(group.banksMapByMint.values())
        .flat()
        .map((bank) => bank.publicKey),
    );
    await extendTable(
      client,
      group,
      payer,
      'token bank oracles',
      [altAddress0, altAddress11],
      Array.from(group.banksMapByMint.values())
        .flat()
        .map((bank) => bank.oracle),
    );
    await extendTable(
      client,
      group,
      payer,
      'token bank vaults',
      [altAddress0, altAddress11],
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
      [altAddress0, altAddress11],
      Array.from(group.perpMarketsMapByMarketIndex.values())
        .flat()
        .map((perpMarket) => [perpMarket.publicKey, perpMarket.oracle])
        .flat(),
    );
    // Well known addresses
    await extendTable(
      client,
      group,
      payer,
      'well known addresses',
      [altAddress0, altAddress11],
      [
        // Solana specific
        SystemProgram.programId,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
        NATIVE_MINT,
        SYSVAR_INSTRUCTIONS_PUBKEY,
        ComputeBudgetProgram.programId,
        // Misc.
        OPENBOOK_PROGRAM_ID['mainnet-beta'],
      ],
    );

    //
    // Table 1 - everything else
    //
    //   const altAddress1 = group.addressLookupTablesList[1].key;
    const altAddress1 = new PublicKey(
      'FGZCgVhVGqzfWnmJFP9Hx4BvGvnFApEp1dM2whzXvg1Z',
    );
    const altAddress2 = new PublicKey(
      'FsruqicZDGnCnm7dRthjL5eFrTmaNRkkomxhPJQP2kdu',
    );
    const altAddress3 = new PublicKey(
      '2JAg3Rm6TmQ3gSYgUCCyZ9bCQKThD9jxHCN6U2ByTPMb',
    );
    const altAddress4 = new PublicKey(
      'BaoRgLAykJovr2Y7BgtPg7rDmkvyp6sG59uJx5wzXTZE',
    );
    // bank mints
    await extendTable(
      client,
      group,
      payer,
      'token mints',
      [altAddress1, altAddress2, altAddress3, altAddress4],
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
      [altAddress1, altAddress2, altAddress3, altAddress4],
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
      [altAddress1, altAddress2, altAddress3, altAddress4],
      Array.from(group.serum3MarketsMapByMarketIndex.values())
        .flat()
        .map((serum3Market) => serum3Market.publicKey),
    );
    await extendTable(
      client,
      group,
      payer,
      'serum3 external markets',
      [altAddress1, altAddress2, altAddress3, altAddress4],
      Array.from(group.serum3ExternalMarketsMap.values())
        .flat()
        .map((serum3ExternalMarket) => serum3ExternalMarket.publicKey),
    );
    await extendTable(
      client,
      group,
      payer,
      'serum3 external markets bids',
      [altAddress1, altAddress2, altAddress3, altAddress4],
      Array.from(group.serum3ExternalMarketsMap.values())
        .flat()
        .map((serum3ExternalMarket) => serum3ExternalMarket.bidsAddress),
    );
    await extendTable(
      client,
      group,
      payer,
      'serum3 external markets asks',
      [altAddress1, altAddress2, altAddress3, altAddress4],
      Array.from(group.serum3ExternalMarketsMap.values())
        .flat()
        .map((serum3ExternalMarket) => serum3ExternalMarket.asksAddress),
    );
    await extendTable(
      client,
      group,
      payer,
      'perp market event queues, bids, and asks',
      [altAddress1, altAddress2, altAddress3, altAddress4],
      Array.from(group.perpMarketsMapByMarketIndex.values())
        .flat()
        .map((perpMarket) => [
          perpMarket.eventQueue,
          perpMarket.bids,
          perpMarket.asks,
        ])
        .flat(),
    );

    const altAddress21 = new PublicKey(
      'BeJQmG5CC4XFc24StGjrE5tD7xbU1mYaofvXu2NiPxaT',
    );
    await extendTable(
      client,
      group,
      payer,
      'sb on demand oracles',
      [altAddress21],
      await buildSbOnDemandAccountsForAlts(connection, group),
    );
  } catch (error) {
    console.log(error);
  }
}

async function populateAltsForSbodOracles(): Promise<void> {
  try {
    const options = AnchorProvider.defaultOptions();
    const connection = new Connection(MB_CLUSTER_URL!, options);
    const payer = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER3_KEYPAIR!, 'utf-8'))),
    );
    const payerWallet = new Wallet(payer);
    const userProvider = new AnchorProvider(connection, payerWallet, options);
    const client = await MangoClient.connect(
      userProvider,
      CLUSTER,
      MANGO_V4_ID[CLUSTER],
      {
        idsSource: 'api',
      },
    );
    const group = await client.getGroup(MANGO_V4_MAIN_GROUP);

    SBOD_ORACLE_LUTS.forEach(async (altAddress) => {
      await extendTable(
        client,
        group,
        payer,
        'sb on demand oracles',
        [new PublicKey(altAddress)],
        await buildSbOnDemandAccountsForAlts(connection, group),
      );
    });
  } catch (error) {
    console.log(error);
  }
}

// uncomment to create a new alt, paste this pubkey in the populate methods, go...
// createANewAlt();

// run the script to populate existing alts
// populateExistingAltsWithMangoGroupAccounts();
populateAltsForSbodOracles();
