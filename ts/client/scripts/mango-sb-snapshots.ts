import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Commitment, Connection, Keypair, PublicKey } from '@solana/web3.js';
import * as fs from 'fs';
import { MANGO_V4_ID, MangoAccount, MangoClient, TokenIndex } from '../src';

const outputCsvName = 'mango-switchboard-snapshot.csv';

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
    MANGO_V4_ID[CLUSTER],
    { idsSource: 'api' },
  );

  const groupKey = new PublicKey(
    '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX',
  );
  const group = await client.getGroup(groupKey);
  const mangoAccounts = await client.getAllMangoAccounts(group, false);

  const sbTokenIndexes = [
    // https://api.mngo.cloud/data/v4/group-metadata
    791, // NOS
    916, // STEP
    881, // GECKO
    889, // Moutai
    743, // JLP
    669, // GUAC
    455, // DUAL
    616, // ALL
    848, // WEN
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
