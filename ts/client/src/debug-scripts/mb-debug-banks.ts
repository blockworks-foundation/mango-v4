import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { coder } from '@project-serum/anchor/dist/cjs/spl/token';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { ZERO_I80F48 } from '../accounts/I80F48';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );

  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  const group = await client.getGroupForCreator(admin.publicKey, 0);
  console.log(`Group ${group.publicKey.toBase58()}`);

  const banks = await client.getBanksForGroup(group);
  const banksMapUsingTokenIndex = new Map(
    banks.map((bank) => {
      (bank as any).indexedDepositsByMangoAccounts = ZERO_I80F48;
      (bank as any).indexedBorrowsByMangoAccounts = ZERO_I80F48;
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
            banksMapUsingTokenIndex.get(token.tokenIndex).depositIndex,
          ),
        );
      }
      if (token.indexedPosition.isNeg()) {
        (bank as any).indexedBorrowsByMangoAccounts = (
          bank as any
        ).indexedBorrowsByMangoAccounts.add(
          token.indexedPosition
            .abs()
            .mul(banksMapUsingTokenIndex.get(token.tokenIndex).borrowIndex),
        );
      }
    }),
  );

  for (const bank of await Array.from(banksMapUsingTokenIndex.values()).sort(
    (a, b) => a.tokenIndex - b.tokenIndex,
  )) {
    let res = `${bank.name}`;
    res =
      res +
      `\n ${'collectedFeesNative'.padEnd(40)} ${bank.collectedFeesNative}` +
      `\n ${'deposits'.padEnd(40)} ${bank.indexedDeposits.mul(
        bank.depositIndex,
      )}` +
      `\n ${'deposits (sum over all mango accounts)'.padEnd(40)} ${
        (bank as any).indexedDepositsByMangoAccounts
      }` +
      `\n ${'cachedIndexedTotalDeposits'.padEnd(40)} ${(
        bank as any
      ).cachedIndexedTotalDeposits.mul(bank.depositIndex)}` +
      `\n ${'indexedBorrows'.padEnd(40)} ${bank.indexedBorrows.mul(
        bank.borrowIndex,
      )}` +
      `\n ${'borrows (sum over all mango accounts)'.padEnd(40)} ${
        (bank as any).indexedBorrowsByMangoAccounts
      }` +
      `\n ${'cachedIndexedTotalBorrows'.padEnd(40)} ${(
        bank as any
      ).cachedIndexedTotalBorrows.mul(bank.borrowIndex)}` +
      `\n ${'avgUtilization'.padEnd(40)} ${bank.avgUtilization}` +
      `\n ${'depositRate'.padEnd(40)} ${bank.getDepositRate()}` +
      `\n ${'borrowRate'.padEnd(40)} ${bank.getBorrowRate()}` +
      `\n ${'vault'.padEnd(40)} ${coder()
        .accounts.decode(
          'token',
          await (
            await client.program.provider.connection.getAccountInfo(bank.vault)
          ).data,
        )
        .amount.toNumber()}`;

    console.log(`${res}`);
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
