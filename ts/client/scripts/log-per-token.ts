import { MangoClient } from '../src/client';
import { MANGO_V4_MAIN_GROUP as MANGO_V4_PRIMARY_GROUP } from '../src/constants';

const { MB_CLUSTER_URL } = process.env;

async function withdrawFeesToAdmin(): Promise<void> {
  const client = await MangoClient.connectDefault(MB_CLUSTER_URL!);
  const group = await client.getGroup(MANGO_V4_PRIMARY_GROUP);

  Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    .sort((a, b) => a.name.localeCompare(b.name))
    .forEach(async (bank) => {
      if (bank.reduceOnly == 1 || bank.uiDeposits() == 0) {
        return;
      }

      console.log(
        `${bank.name.padStart(20)},  ${(
          bank.collateralFeePerDay *
          365 *
          100
        ).toFixed(2)}, ${(1 / (1 - bank.initAssetWeight.toNumber())).toFixed(
          2,
        )}, ${(1 / (1 - bank.maintAssetWeight.toNumber())).toFixed(2)} `,
      );
    });
}

async function main(): Promise<void> {
  try {
    await withdrawFeesToAdmin();
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
