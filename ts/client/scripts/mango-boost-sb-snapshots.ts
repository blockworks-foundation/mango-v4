import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Commitment, Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as fs from 'fs';
import { MangoAccount, MangoClient, TokenIndex } from '../src';

const outputCsvName = 'mango-boost-switchboard-snapshot.csv';

const script = async () => {
  const connection = new Connection(
    process.env.MB_CLUSTER_URL!,
    'confirmed' as Commitment,
  );
  const kp = Keypair.generate();
  const wallet = new Wallet(kp);
  const provider = new AnchorProvider(connection, wallet, {});

  const CLUSTER = 'mainnet-beta';
  const client = MangoClient.connect(
    provider as any,
    CLUSTER,
    new PublicKey('zF2vSz6V9g1YHGmfrzsY497NJzbRr84QUrPry4bLQ25'),
    { idsSource: 'get-program-accounts' },
  );

  const groupKey = new PublicKey(
    'AKeMSYiJekyKfwCc3CUfVNDVAiqk9FfbQVMY3G7RUZUf',
  );
  const group = await client.getGroup(groupKey);
  const mangoAccounts = await client.getAllMangoAccounts(group, false);

  const sbTokenIndexes = [
    1, // JLP
  ];

  const sbBanks = await Promise.all(
    sbTokenIndexes.map((i) => group.getFirstBankByTokenIndex(i as TokenIndex)),
  );

  function accountHasSbTokens(m: MangoAccount): boolean {
    for (const bank of sbBanks) {
      if (m.getTokenBalanceUi(bank) !== 0) {
        return true;
      }
    }
    return false;
  }
  const sbAccounts = mangoAccounts.filter((m) => accountHasSbTokens(m));

  const resultMap = new Map<string, [number, number]>();
  for (const account of sbAccounts) {
    let deposits = 0;
    let borrows = 0;

    for (const bank of sbBanks) {
      deposits += account.getTokenDepositsUi(bank) * bank.uiPrice;
      borrows += account.getTokenBorrowsUi(bank) * bank.uiPrice;
    }

    const wallet = account.owner.toBase58();

    const existingValue = resultMap.get(account.owner.toBase58());
    if (existingValue) {
      const [d, b] = existingValue;
      resultMap.set(wallet, [d + deposits, b + borrows]);
    } else {
      resultMap.set(wallet, [deposits, borrows]);
    }
  }

  // console.log(resultMap.entries())
  writeMapToCsv(resultMap, outputCsvName);
};

script();

function writeMapToCsv(
  map: Map<string, [number, number]>,
  filename: string,
): void {
  let csv = 'wallet,deposits_value,borrows_value\n';
  for (let [wallet, [deposits, borrows]] of map) {
    csv += `${wallet},${deposits},${borrows}\n`;
  }

  fs.writeFileSync(filename, csv);
}
