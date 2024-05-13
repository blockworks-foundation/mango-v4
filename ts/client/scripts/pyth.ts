import { OracleProvider } from '../src/accounts/oracle';
import { MangoClient } from '../src/client';
import { MANGO_V4_MAIN_GROUP as MANGO_V4_PRIMARY_GROUP } from '../src/constants';

const { MB_CLUSTER_URL } = process.env;

async function buildClient(): Promise<MangoClient> {
  return await MangoClient.connectDefault(MB_CLUSTER_URL!);
}

async function updateSpotMarkets(): Promise<void> {
  const [client] = await Promise.all([buildClient()]);

  const group = await client.getGroup(MANGO_V4_PRIMARY_GROUP);

  Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    .sort((a, b) => a.name.localeCompare(b.name))
    .forEach(async (bank) => {
      // https://pyth.network/developers/price-feed-ids, use pyth evm stable
      // https://docs.pyth.network/price-feeds/sponsored-feeds

      const sponsored = [
        'SOL',
        'JITOSOL',
        'MSOL',
        'BSOL',
        'BONK',
        'W',
        'KMNO',
        'MEW',
        'TNSR',
        'USDC',
        'BTC',
        'JTO',
        'USDT',
        'JUP',
        'ETH',
        'PYTH',
        'HNT',
        'RNDR',
        'ORCA',
        'SAMO',
        'WIF',
        'LST',
        'INF',
        'PRCL',
        'RAY',
        'FIDA',
        'MNDE',
        'MOBILE',
        'IOT',
        'GOFX',
        'NEON',
        'AUD',
        'GBP',
        'EUR',
        'XAG',
        'XAU',
      ];

      if (bank.oracleProvider == OracleProvider.Pyth) {
        let bankName = bank.name;
        if (bankName == 'ETH (Portal)') {
          bankName = 'ETH';
        }
        if (bankName == 'wBTC (Portal)') {
          bankName = 'BTC';
        }

        console.log(
          `${bank.name}, ${bank.oracle}, is sponsored ${
            sponsored.filter((t) => t.toLowerCase() == bankName.toLowerCase())
              .length > 0
          }`,
        );
      }
    });
}

async function main(): Promise<void> {
  try {
    await updateSpotMarkets();
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
