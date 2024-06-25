import { parsePriceData } from '@pythnetwork/client';
import { Connection, PublicKey } from '@solana/web3.js';
import {
  isPythOracle,
  isSwitchboardOracle,
  parseSwitchboardOracle,
} from '../../src/accounts/oracle';
import { toNativeI80F48 } from '../../src/utils';
const { MB_CLUSTER_URL } = process.env;

async function decodePrice(conn, ai, pk): Promise<void> {
  let uiPrice, price, lastUpdatedSlot, type;
  if (isPythOracle(ai!)) {
    const priceData = parsePriceData(ai!.data);
    uiPrice = priceData.previousPrice;
    price = toNativeI80F48(uiPrice, 6 - 5);
    lastUpdatedSlot = parseInt(priceData.lastSlot.toString());
    type = 'pyth';
  } else if (isSwitchboardOracle(ai!)) {
    const priceData = await parseSwitchboardOracle(pk, ai!, conn);
    uiPrice = priceData.price;
    price = toNativeI80F48(uiPrice, 6 - 5);
    lastUpdatedSlot = priceData.lastUpdatedSlot;
    type = 'sb';
  }
  console.log(`type ${type}`);
  console.log(`uiPrice ${uiPrice}`);
  console.log(`price ${price}`);
  console.log(`lastUpdatedSlot ${lastUpdatedSlot}`);
}

async function main(): Promise<void> {
  try {
    // {
    //   const oraclePk1 = new PublicKey(
    //     '4SZ1qb4MtSUrZcoeaeQ3BDzVCyqxw3VwSFpPiMTmn4GE',
    //   );
    //   const conn = new Connection(MB_CLUSTER_URL!);
    //   let ai = await conn.getAccountInfo(oraclePk1);
    //   decodePrice(conn, ai, oraclePk1);

    //   const oraclePk2 = new PublicKey(
    //     '8ihFLu5FimgTQ1Unh4dVyEHUGodJ5gJQCrQf4KUVB9bN',
    //   );
    //   ai = await conn.getAccountInfo(oraclePk2);
    //   decodePrice(conn, ai, oraclePk2);
    // }

    {
      // https://ondemand.switchboard.xyz/solana/devnet/feed/23QLa7R2hDhcXDVKyUSt2rvBPtuAAbY44TrqMVoPpk1C
      const oraclePk3 = new PublicKey(
        'EtbG8PSDCyCSmDH8RE4Nf2qTV9d6P6zShzHY2XWvjFJf',
      );
      const devnetConn = new Connection(process.env.DEVNET_CLUSTER_URL!);
      const ai = await devnetConn.getAccountInfo(oraclePk3);
      decodePrice(devnetConn, ai, oraclePk3);
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
