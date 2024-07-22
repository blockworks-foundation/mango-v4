import { Connection, PublicKey } from '@solana/web3.js';
import {
  isPythOracle,
  isSwitchboardOracle,
  parsePythOracle,
  parseSwitchboardOracle,
} from '../../src/accounts/oracle';
import { SB_ON_DEMAND_TESTING_ORACLES } from '../governanceInstructions/constants';
const { MB_CLUSTER_URL } = process.env;

async function decodePrice(slot, name, conn, ai, pk): Promise<void> {
  let uiPrice, price, lastUpdatedSlot, type, uiDeviation;

  if (isPythOracle(ai!)) {
    const priceData = parsePythOracle(ai!, conn);
    uiPrice = priceData.price;
    lastUpdatedSlot = priceData.lastUpdatedSlot;
    uiDeviation = priceData.uiDeviation;
    type = 'pyth';
  } else if (isSwitchboardOracle(ai!)) {
    const priceData = await parseSwitchboardOracle(pk, ai!, conn);
    uiPrice = priceData.price;
    uiDeviation = priceData.uiDeviation;
    lastUpdatedSlot = priceData.lastUpdatedSlot;
    type = 'sb';
  }
  console.log(
    `${name.toString().padStart(10)}, ${type.padStart(4)}, ${uiPrice
      .toString()
      .padStart(10)}, ${(slot - lastUpdatedSlot) / 2}s  ${uiDeviation
      .toString()
      .padStart(10)}`,
  );
}

async function main(): Promise<void> {
  try {
    const conn = new Connection(MB_CLUSTER_URL!);

    if (true) {
      {
        // https://ondemand.switchboard.xyz/solana/mainnet/user/DrnFiKkbyC5ga7LJDfDF8FzVcj6aoSUhsgirLjDMrBHH

        for (const item of SB_ON_DEMAND_TESTING_ORACLES) {
          const oraclePk = new PublicKey(item[1]);
          const slot = await conn.getSlot();
          const ai = await conn.getAccountInfo(oraclePk);
          decodePrice(slot, item[0], conn, ai, oraclePk);
        }
      }

      // eslint-disable-next-line no-constant-condition
      if (false) {
        // https://docs.pyth.network/price-feeds/sponsored-feeds
        for (const item of [
          ['SOL/USD', '7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE'],
          ['JITOSOL/USD', 'AxaxyeDT8JnWERSaTKvFXvPKkEdxnamKSqpWbsSjYg1g'],
          ['MSOL/USD', '5CKzb9j4ChgLUt8Gfm5CNGLN6khXKiqMbnGAW4cgXgxK'],
          ['BSOL/USD', '5cN76Xm2Dtx9MnrQqBDeZZRsWruTTcw37UruznAdSvvE'],
          ['BONK/USD', 'DBE3N8uNjhKPRHfANdwGvCZghWXyLPdqdSbEW2XFwBiX'],
          ['W/USD', 'BEMsCSQEGi2kwPA4mKnGjxnreijhMki7L4eeb96ypzF9'],
          ['KMNO/USD', 'ArjngUHXrQPr1wH9Bqrji9hdDQirM6ijbzc1Jj1fXUk7'],
          ['MEW/USD', 'EF6U755BdHMXim8RBw6XSC6Yk6XaouTKpwcBZ7QkcanB'],
          ['TNSR/USD', '9TSGDwcPQX4JpAvZbu2Wp5b68wSYkQvHCvfeBjYcCyC'],
          ['USDC/USD', 'Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX'],
          ['BTC/USD', '4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo'],
          ['JTO/USD', '7ajR2zA4MGMMTqRAVjghTKqPPn4kbrj3pYkAVRVwTGzP'],
          ['USDT/USD', 'HT2PLQBcG5EiCcNSaMHAjSgd9F98ecpATbk4Sk5oYuM'],
          ['JUP/USD', '7dbob1psH1iZBS7qPsm3Kwbf5DzSXK8Jyg31CTgTnxH5'],
          ['ETH/USD', '42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC'],
          ['PYTH/USD', '8vjchtMuJNY4oFQdTi8yCe6mhCaNBFaUbktT482TpLPS'],
          ['HNT/USD', '4DdmDswskDxXGpwHrXUfn2CNUm9rt21ac79GHNTN3J33'],
          ['RNDR/USD', 'GbgH1oen3Ne1RY4LwDgh8kEeA1KywHvs5x8zsx6uNV5M'],
          ['ORCA/USD', '4CBshVeNBEXz24GZpoj8SrqP5L7VGG3qjGd6tCST1pND'],
          ['SAMO/USD', '2eUVzcYccqXzsDU1iBuatUaDCbRKBjegEaPPeChzfocG'],
          ['WIF/USD', '6B23K3tkb51vLZA14jcEQVCA1pfHptzEHFA93V5dYwbT'],
          ['LST/USD', '7aT9A5knp62jVvnEW33xaWopaPHa3Y7ggULyYiUsDhu8'],
          ['INF/USD', 'Ceg5oePJv1a6RR541qKeQaTepvERA3i8SvyueX9tT8Sq'],
          ['PRCL/USD', '6a9HN13ZFf57WZd4msn85KWLe5iTayqS8Ee8gstQkxqm'],
          ['RAY/USD', 'Hhipna3EoWR7u8pDruUg8RxhP5F6XLh6SEHMVDmZhWi8'],
          ['FIDA/USD', '2cfmeuVBf7bvBJcjKBQgAwfvpUvdZV7K8NZxUEuccrub'],
          ['MNDE/USD', 'GHKcxocPyzSjy7tWApQjKRkDNuVXd4Kk624zhuaR7xhC'],
          ['MOBILE/USD', 'DQ4C1tzvu28cwo1roN1Wm6TW35sfJEjLh517k3ZeWevx'],
          ['IOT/USD', '8UYEn5Weq7toHwgcmctvcAxaNJo3SJxXEayM57rpoXr9'],
          ['GOFX/USD', '2WS7DByXgzmsGD1QfDyvY2pwAmxjsPDrF2DijwpRBxr7'],
          ['NEON/USD', 'F2VfCymdNQiCa8Vyg5E7BwEv9UPwfm8cVN6eqQLqXiGo'],
          ['AUD/USD', '6pPXqXcgFFoLEcXfedWJy3ypNZVJ1F3mgipaDFsvZ1co'],
          ['GBP/USD', 'G25Tm7UkVruTJ7mcbCxFm45XGWwsH72nJKNGcHEQw1tU'],
          ['EUR/USD', 'Fu76ChamBDjE8UuGLV6GP2AcPPSU6gjhkNhAyuoPm7ny'],
          ['XAG/USD', 'H9JxsWwtDZxjSL6m7cdCVsWibj3JBMD9sxqLjadoZnot'],
          ['XAU/USD', '2uPQGpm8X4ZkxMHxrAW1QuhXcse1AHEgPih6Xp9NuEWW'],
          ['INJ/USD', 'GwXYEfmPdgHcowF9GZwbb1WiTGTn1fuT3hbSLneoBKK6'],
          ['SLND/USD', '6vPfd6612huknxXaDapfj6cVmB8NvCwKm3BHKFxzo1EZ'],
        ]) {
          const oraclePk = new PublicKey(item[1]);
          const slot = await conn.getSlot();
          const ai = await conn.getAccountInfo(oraclePk);
          decodePrice(slot, item[0], conn, ai, oraclePk);
        }
      }
    }
  } catch (error) {
    console.log(error);
  }
}

try {
  main();
} catch (error) {
  console.log(error);
}
