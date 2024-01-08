import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { getAccount } from '@solana/spl-token';
import { Cluster, Connection, Keypair } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fs from 'fs';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { I80F48, ZERO_I80F48 } from '../src/numbers/I80F48';
import { toUiDecimals } from '../src/utils';
dotenv.config();

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const PAYER_KEYPAIR =
  process.env.PAYER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_NUM = Number(process.env.GROUP_NUM || 0);
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
    { idsSource: 'get-program-accounts' },
  );

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`Group ${group.publicKey.toBase58()}`);

  const banks = Array.from(group.banksMapByMint.values()).flat();
  const banksMapUsingTokenIndex = new Map(
    banks.map((bank) => {
      (bank as any).indexedDepositsByMangoAccounts = ZERO_I80F48();
      (bank as any).indexedBorrowsByMangoAccounts = ZERO_I80F48();
      (bank as any).serum3Total = ZERO_I80F48();
      return [bank.tokenIndex, bank];
    }),
  );

  const mangoAccounts = await client.getAllMangoAccounts(group, true);

  mangoAccounts.map((mangoAccount) => {
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
    });

    mangoAccount.serum3Active().map((s3a) => {
      const baseBank = group.getFirstBankByTokenIndex(s3a.baseTokenIndex);
      const quoteBank = group.getFirstBankByTokenIndex(s3a.quoteTokenIndex);

      const oo = mangoAccount.serum3OosMapByMarketIndex.get(s3a.marketIndex);
      (baseBank as any).serum3Total = (baseBank as any).serum3Total.add(
        I80F48.fromU64(oo!.baseTokenTotal),
      );
      (quoteBank as any).serum3Total = (quoteBank as any).serum3Total.add(
        I80F48.fromU64(oo!.quoteTokenTotal),
      );
    });
  });

  for (const bank of await Array.from(banksMapUsingTokenIndex.values()).sort(
    (a, b) => a.tokenIndex - b.tokenIndex,
  )) {
    const account = await getAccount(
      client.program.provider.connection,
      bank.vault,
    );
    const vault = I80F48.fromNumber(Number(account.amount));

    const error = vault
      .sub(
        bank.indexedDeposits
          .mul(bank.depositIndex)
          .sub(bank.indexedBorrows.mul(bank.borrowIndex)),
      )
      .sub(bank.collectedFeesNative)
      .sub(bank.dust)
      .add(I80F48.fromU64(bank.feesWithdrawn));
    let res = `${bank.name}`;
    res =
      res +
      `\n ${'error'.padEnd(40)} ${toUiDecimals(
        error,
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'vault'.padEnd(40)} ${toUiDecimals(
        vault,
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'collected fees'.padEnd(40)} ${toUiDecimals(
        bank.collectedFeesNative,
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'fees withdrawn'.padEnd(40)} ${toUiDecimals(
        bank.feesWithdrawn,
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'deposits'.padEnd(40)} ${toUiDecimals(
        bank.indexedDeposits.mul(bank.depositIndex),
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'deposits (sum over all mango accounts)'.padEnd(40)} ${toUiDecimals(
        (bank as any).indexedDepositsByMangoAccounts,
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'borrows'.padEnd(40)} ${toUiDecimals(
        bank.indexedBorrows.mul(bank.borrowIndex),
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'borrows (sum over all mango accounts)'.padEnd(40)} ${toUiDecimals(
        (bank as any).indexedBorrowsByMangoAccounts,
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${'deposits - borrows'.padEnd(40)} ${toUiDecimals(
        bank.indexedDeposits
          .mul(bank.depositIndex)
          .sub(bank.indexedBorrows.mul(bank.borrowIndex)),
        bank.mintDecimals,
      ).toLocaleString()}` +
      `\n ${`serum3 total`.padEnd(40)} ${toUiDecimals(
        (bank as any).serum3Total,
        bank.mintDecimals,
      ).toLocaleString()}`;

    console.log(`${res}`);
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
