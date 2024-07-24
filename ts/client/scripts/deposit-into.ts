import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { BN } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { ZERO_I80F48 } from '../src/numbers/I80F48';
import { toUiDecimalsForQuote } from '../src/utils';

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, MANGO_ACCOUNT, MINT, NATIVE_AMOUNT } =
  process.env;

const CLIENT_USER = MB_PAYER_KEYPAIR;
const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

async function buildClient(): Promise<MangoClient> {
  const clientKeypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(CLIENT_USER!, 'utf-8'))),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const clientWallet = new Wallet(clientKeypair);
  const clientProvider = new AnchorProvider(connection, clientWallet, options);

  return await MangoClient.connect(
    clientProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );
}

async function main(): Promise<void> {
  const client = await buildClient();
  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const mangoAccounts = await client.getAllMangoAccounts(group, true);

  for (const bank of Array.from(group.banksMapByName.values()).flat()) {
    if (bank.uiDeposits() * bank.uiPrice > 10) continue;

    if (bank.nativeDeposits().eq(ZERO_I80F48())) continue;

    if (bank.reduceOnly != 1) continue;

    console.log(`${bank.name}, ${bank.uiDeposits()}`);
    for (const mangoAccount of mangoAccounts) {
      if (mangoAccount.getTokenBalance(bank).lt(ZERO_I80F48())) {
        console.log(
          `${bank.name}, ${toUiDecimalsForQuote(
            mangoAccount.getEquity(group),
          )} ${mangoAccount.publicKey}, ${mangoAccount.getTokenBalance(
            bank,
          )}, ${mangoAccount.getTokenBalance(bank).ceil().toNumber()}`,
        );

        const rs = await client.tokenDepositNative(
          group,
          mangoAccount,
          bank.mint,
          new BN(mangoAccount.getTokenBalance(bank).ceil().toNumber()),
          false,
          true,
        );
        console.log(rs.signature);
      }

      if (mangoAccount.getTokenBalance(bank).gt(ZERO_I80F48())) {
        console.log(
          `${bank.name}, ${toUiDecimalsForQuote(
            mangoAccount.getEquity(group),
          )} ${mangoAccount.publicKey}, ${mangoAccount.getTokenBalance(
            bank,
          )}, ${mangoAccount.getTokenBalance(bank).ceil().toNumber()}, ${
            mangoAccount.getToken(bank.tokenIndex)?.inUseCount
          }`,
        );

        const rs = await client.tokenForceWithdraw(
          group,
          mangoAccount,
          bank.tokenIndex,
        );
        console.log(rs.signature);
      }
    }
    console.log('');
  }
  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
