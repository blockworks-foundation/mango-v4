import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { coder } from '@project-serum/anchor/dist/cjs/spl/token';
import { Cluster, Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { toUiDecimals } from '../utils';

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const PAYER_KEYPAIR =
  process.env.PAYER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_NUM = Number(process.env.GROUP_NUM || 2);
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(PAYER_KEYPAIR!, 'utf-8'))),
  );

  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = MangoClient.connect(
    adminProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {},
    'get-program-accounts',
  );

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`Group ${group.publicKey.toBase58()}`);
  console.log(`${group.toString()}`);

  const banks = Array.from(group.banksMapByMint.values()).flat();
  const banksMapUsingTokenIndex = new Map(
    banks.map((bank) => {
      (bank as any).indexedDepositsByMangoAccounts = ZERO_I80F48();
      (bank as any).indexedBorrowsByMangoAccounts = ZERO_I80F48();
      return [bank.tokenIndex, bank];
    }),
  );

  const mangoAccounts = await client.getAllMangoAccounts(group);
  mangoAccounts.forEach((mangoAccount) =>
    console.log(
      `MangoAccount pk - ${mangoAccount.publicKey}, owner - ${mangoAccount.owner}`,
    ),
  );
  mangoAccounts.map((mangoAccount) =>
    mangoAccount.tokensActive().forEach((token) => {
      const bank = banksMapUsingTokenIndex.get(token.tokenIndex);
      if (token.indexedPosition.isPos()) {
        (bank as any).indexedDepositsByMangoAccounts = (
          bank as any
        ).indexedDepositsByMangoAccounts.add(
          token.indexedPosition.mul(
            banksMapUsingTokenIndex.get(token.tokenIndex)!.depositIndex,
          ),
        );
      }
      if (token.indexedPosition.isNeg()) {
        (bank as any).indexedBorrowsByMangoAccounts = (
          bank as any
        ).indexedBorrowsByMangoAccounts.add(
          token.indexedPosition
            .abs()
            .mul(banksMapUsingTokenIndex.get(token.tokenIndex)!.borrowIndex),
        );
      }
    }),
  );

  for (const bank of await Array.from(banksMapUsingTokenIndex.values()).sort(
    (a, b) => a.tokenIndex - b.tokenIndex,
  )) {
    const vault = I80F48.fromNumber(
      coder()
        .accounts.decode(
          'token',
          (await client.program.provider.connection.getAccountInfo(bank.vault))!
            .data,
        )
        .amount.toNumber(),
    );

    const error = vault.sub(
      (bank as any).indexedDepositsByMangoAccounts
        .sub((bank as any).indexedBorrowsByMangoAccounts)
        .add(bank.collectedFeesNative)
        .add(bank.dust),
    );

    let res = `${bank.name}`;
    res =
      res +
      `\n ${'tokenIndex'.padEnd(40)} ${bank.tokenIndex}` +
      `\n ${'bank'.padEnd(40)} ${bank.publicKey}` +
      `\n ${'vault'.padEnd(40)} ${bank.vault}` +
      `\n ${'mint'.padEnd(40)} ${bank.mint}` +
      `\n ${'price'.padEnd(40)} ${bank.price?.toNumber()}` +
      `\n ${'uiPrice'.padEnd(40)} ${bank.uiPrice}` +
      `\n ${'error'.padEnd(40)} ${error}` +
      `\n ${'collectedFeesNative'.padEnd(40)} ${bank.collectedFeesNative}` +
      `\n ${'dust'.padEnd(40)} ${bank.dust}` +
      `\n ${'deposits'.padEnd(40)} ${bank.indexedDeposits.mul(
        bank.depositIndex,
      )}` +
      `\n ${'deposits (sum over all mango accounts)'.padEnd(40)} ${
        (bank as any).indexedDepositsByMangoAccounts
      }` +
      `\n ${'cachedTotalDeposits'.padEnd(40)} ${(
        bank as any
      ).cachedIndexedTotalDeposits.mul(bank.depositIndex)}` +
      `\n ${'borrows'.padEnd(40)} ${bank.indexedBorrows.mul(
        bank.borrowIndex,
      )}` +
      `\n ${'borrows (sum over all mango accounts)'.padEnd(40)} ${
        (bank as any).indexedBorrowsByMangoAccounts
      }` +
      `\n ${'cachedTotalBorrows'.padEnd(40)} ${(
        bank as any
      ).cachedIndexedTotalBorrows.mul(bank.borrowIndex)}` +
      `\n ${'avgUtilization since last rate update'.padEnd(40)} ${(
        100 * bank.avgUtilization.toNumber()
      ).toFixed(1)}%` +
      `\n ${'rate parameters'.padEnd(40)} ${(
        100 * bank.rate0.toNumber()
      ).toFixed()}% @ ${(100 * bank.util0.toNumber()).toFixed()}% util, ${(
        100 * bank.rate1.toNumber()
      ).toFixed()}% @${(100 * bank.util1.toNumber()).toFixed()}% util, ${(
        100 * bank.maxRate.toNumber()
      ).toFixed()}% @ 100% util` +
      `\n ${'depositRate'.padEnd(40)} ${(
        100 * bank.getDepositRate().toNumber()
      ).toFixed(2)}%` +
      `\n ${'borrowRate'.padEnd(40)} ${(
        100 * bank.getBorrowRate().toNumber()
      ).toFixed(2)}%` +
      `\n ${'vault balance'.padEnd(40)} ${toUiDecimals(
        vault,
        bank.mintDecimals,
      )}, ${vault} native` +
      `\n ${'last index update'.padEnd(40)} ${new Date(
        1000 * bank.indexLastUpdated.toNumber(),
      )}` +
      `\n ${'last rates update'.padEnd(40)} ${new Date(
        1000 * bank.bankRateLastUpdated.toNumber(),
      )}`;

    console.log(`${res}`);
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
