import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  AccountInfo,
  Cluster,
  Connection,
  Keypair,
  PublicKey,
} from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../src/accounts/bank';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import {
  fetchJupiterTransaction,
  fetchRoutes,
  prepareMangoRouterInstructions,
} from '../src/router';
import { toNative, toUiDecimals } from '../src/utils';
import * as borsh from '@coral-xyz/borsh';
import { TokenConditionalSwapDto } from '../src/accounts/mangoAccount';

const CLUSTER: Cluster = (process.env.CLUSTER as Cluster) || 'mainnet-beta';
const CLUSTER_URL = process.env.CLUSTER_URL;
const PAYER_KEYPAIR = process.env.PAYER_KEYPAIR;

async function run(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(PAYER_KEYPAIR!, 'utf-8'))),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  //
  // Retrieve accounts
  //
  const discriminatorMemcmp: {
    offset: number;
    bytes: string;
  } = client.program.account.mangoAccount.coder.accounts.memcmp(
    'mangoAccount',
    undefined,
  );

  const accountAis =
    await client.program.provider.connection.getProgramAccounts(
      client.programId,
      {
        filters: [
          {
            memcmp: {
              bytes: discriminatorMemcmp.bytes,
              offset: discriminatorMemcmp.offset,
            },
          },
        ],
      },
    );

  console.log('accounts', accountAis.length);

  for (const accountAi of accountAis) {
    const version = getAccountVersion(client, accountAi.account);
    console.log(accountAi.pubkey.toString(), version);

    if (version == 'v3') {
      continue;
    }

    const account = await client.getMangoAccountFromAi(
      accountAi.pubkey,
      accountAi.account,
    );

    const ix = await client.program.methods
      .accountSizeMigration()
      .accounts({
        group: account.group,
        account: accountAi.pubkey,
        payer: (client.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
    await client.sendAndConfirmTransaction([ix]);
  }
}

function getAccountVersion(
  client: MangoClient,
  ai: AccountInfo<Buffer>,
): 'v1' | 'v2' | 'v3' {
  const currentSize = ai.data.length;

  // Decode a v1 account
  const decodedMangoAccount = client.program.coder.accounts.decode(
    'mangoAccount',
    ai.data,
  );

  // v1: basic length before the introduction of tcs
  const mangoAccountBuffer = Buffer.alloc(currentSize);
  const layout =
    client.program.coder.accounts['accountLayouts'].get('mangoAccount');
  const discriminatorLen = 8;
  const v1DataLen = layout.encode(decodedMangoAccount, mangoAccountBuffer);
  const v1Len = discriminatorLen + v1DataLen;

  if (currentSize == v1Len) {
    return 'v1';
  }

  // v2: addition of the tcs vector
  const tcsAlign = 4;
  const tcsLayout = (client.program as any)._coder.types.typeLayouts.get(
    'TokenConditionalSwap',
  );
  const tcsVecLayout = borsh.vec(tcsLayout);
  const tokenConditionalSwaps =
    currentSize > v1Len
      ? (tcsVecLayout.decode(
          ai.data,
          v1Len + tcsAlign,
        ) as TokenConditionalSwapDto[])
      : new Array<TokenConditionalSwapDto>();
  const tcsBytesSize = tcsVecLayout.encode(
    tokenConditionalSwaps,
    mangoAccountBuffer,
    v1Len + tcsAlign,
  );
  const v2DataLen = v1DataLen + tcsAlign + tcsBytesSize;
  const v2Len = discriminatorLen + v2DataLen;

  if (currentSize == v2Len) {
    return 'v2';
  }

  // v3: add 64 reserved bytes after the tcs vec
  const v3Len = v2Len + 64;

  if (currentSize == v3Len) {
    return 'v3';
  }

  throw new Error(
    `unexpected mango account size ${currentSize}, expected ${v1Len} or ${v2Len} or ${v3Len}`,
  );
}

run();
