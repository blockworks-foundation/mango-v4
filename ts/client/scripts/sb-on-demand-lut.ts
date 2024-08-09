import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  AddressLookupTableProgram,
  Cluster,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';
import { PullFeed, SB_ON_DEMAND_PID } from '@switchboard-xyz/on-demand';
import fs from 'fs';
import chunk from 'lodash/chunk';
import { Program as Anchor30Program } from 'switchboard-anchor';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID, MANGO_V4_MAIN_GROUP } from '../src/constants';
import { buildVersionedTx } from '../src/utils';

const { MB_CLUSTER_URL, MB_PAYER3_KEYPAIR, DRY_RUN } = process.env;
const CLUSTER: Cluster = (process.env.CLUSTER as Cluster) || 'mainnet-beta';

// eslint-disable-next-line no-inner-declarations
async function extendTable(
  client: MangoClient,
  payer: Keypair,
  altAddresses: PublicKey[],
  sbOnDemandOracles: [[string, PublicKey]],
): Promise<void> {
  const idl = await Anchor30Program.fetchIdl(
    SB_ON_DEMAND_PID,
    client.program.provider,
  );
  const sbOnDemandProgram = new Anchor30Program(idl!, client.program.provider);

  await Promise.all(
    sbOnDemandOracles.map(async (item) => {
      item[1];

      const pullFeed = new PullFeed(sbOnDemandProgram as any, item[1]);

      const conf = {
        numSignatures: 2,
        feed: item[1],
      };

      const [pullIx, responses, success] = await pullFeed.fetchUpdateIx(conf);
    }),
  );

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
            `...need to create a new alt, all existing ones are full`,
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
    const sig =
      await client.program.provider.connection.sendTransaction(extendTx);
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
      idsSource: 'get-program-accounts',
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
  const sig = await connection.sendTransaction(createTx);
  console.log(
    `...created ALT ${createIx[1]} https://explorer.solana.com/tx/${sig}`,
  );
}

async function populateExistingAlts(): Promise<void> {
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
        idsSource: 'get-program-accounts',
      },
    );
    const group = await client.getGroup(MANGO_V4_MAIN_GROUP);

    const altAddress0 = new PublicKey('');
    await extendTable(
      client,
      payer,
      [altAddress0],
      [
        ['DIGITSOL', '2A7aqNLy26ZBSMWP2Ekxv926hj16tCA47W1sHWVqaLii'],
        ['JLP', '65J9bVEMhNbtbsNgArNV1K4krzcsomjho4bgR51sZXoj'],
        ['INF', 'AZcoqpWhMJUaKEDUfKsfzCr3Y96gSQwv43KSQ6KpeyQ1'],
        ['GUAC', 'Ai2GsLRioGKwVgWX8dtbLF5rJJEZX17SteGEDqrpzBv3'],
        ['RAY', 'AJkAFiXdbMonys8rTXZBrRnuUiLcDFdkyoPuvrVKXhex'],
        ['JUP', '2F9M59yYc28WMrAymNWceaBEk8ZmDAjUAKULp8seAJF3'],
      ],
    );
  } catch (error) {
    console.log(error);
  }
}

// createANewAlt();
populateExistingAlts();
