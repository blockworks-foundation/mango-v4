import {
  Connection,
  ParsedAccountData,
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
} from '@solana/web3.js';
import {
  isPythOracle,
  isSwitchboardOracle,
  parsePythOracle,
  parseSwitchboardOracle,
} from '../../src/accounts/oracle';
import {
  PYTH_SPONSORED_ORACLES,
  SB_ON_DEMAND_LST_FALLBACK_ORACLES,
} from '../governanceInstructions/constants';
const { MB_CLUSTER_URL } = process.env;

async function decodePrice(
  slot,
  name,
  conn: Connection,
  ai,
  pk,
): Promise<void> {
  let uiPrice, price, lastUpdatedSlot, type, uiDeviation, publishedTime;

  if (isPythOracle(ai!)) {
    const priceData = parsePythOracle(ai!, conn);
    uiPrice = priceData.price;
    lastUpdatedSlot = priceData.lastUpdatedSlot;
    uiDeviation = priceData.uiDeviation;
    publishedTime = (priceData as any).publishedTime;
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

  const localUnixTime = Math.floor(Date.now() / 1000);

  const parsedClock = await conn.getParsedAccountInfo(SYSVAR_CLOCK_PUBKEY);
  const parsedClockAccount = (parsedClock.value!.data as ParsedAccountData)
    .parsed;
  const solanaUnixTime = parsedClockAccount.info.unixTimestamp;

  console.log(
    `${name}, ${localUnixTime - solanaUnixTime}, ${
      localUnixTime - publishedTime
    }`,
  );
}

async function main(): Promise<void> {
  try {
    const conn = new Connection(MB_CLUSTER_URL!);

    // eslint-disable-next-line no-constant-condition
    if (true) {
      // https://ondemand.switchboard.xyz/solana/mainnet/user/DrnFiKkbyC5ga7LJDfDF8FzVcj6aoSUhsgirLjDMrBHH

      for (const item of SB_ON_DEMAND_LST_FALLBACK_ORACLES) {
        const oraclePk = new PublicKey(item[1]);
        const slot = await conn.getSlot();
        const ai = await conn.getAccountInfo(oraclePk);
        decodePrice(slot, item[0], conn, ai, oraclePk);
      }
    }

    // eslint-disable-next-line no-constant-condition
    if (true) {
      // https://docs.pyth.network/price-feeds/sponsored-feeds
      for (const item of PYTH_SPONSORED_ORACLES) {
        const oraclePk = new PublicKey(item[1]);
        const slot = await conn.getSlot();
        const ai = await conn.getAccountInfo(oraclePk);
        decodePrice(slot, item[0], conn, ai, oraclePk);
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
