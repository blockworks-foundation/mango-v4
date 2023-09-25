import { PublicKey } from '@solana/web3.js';
import { Bank } from '../src/accounts/bank';
import { Group } from '../src/accounts/group';
import { isSwitchboardOracle } from '../src/accounts/oracle';
import { PerpMarket } from '../src/accounts/perp';
import { MangoClient } from '../src/client';
import { buildFetch } from '../src/utils';

function getBankForOracle(group: Group, oracle: PublicKey): Bank | PerpMarket {
  let match: Bank[] | PerpMarket[] = Array.from(group.banksMapByName.values())
    .flat()
    .filter((b) => b.oracle.equals(oracle));
  if (match.length > 0) {
    return match[0];
  }

  match = Array.from(group.perpMarketsMapByName.values()).filter((p) =>
    p.oracle.equals(oracle),
  );
  if (match.length > 0) {
    return match[0];
  }

  throw new Error(`No token or perp market found for ${oracle}`);
}

async function main(): Promise<void> {
  const client = await MangoClient.connectDefault(process.env.MB_CLUSTER_URL!);
  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const oracles1 = Array.from(group.banksMapByName.values()).map(
    (b) => b[0].oracle,
  );
  const oracles2 = Array.from(group.perpMarketsMapByName.values()).map(
    (p) => p.oracle,
  );
  const oracles = oracles1.concat(oracles2);

  const ais = await client.program.provider.connection.getMultipleAccountsInfo(
    oracles,
  );

  const switcboardOracles: PublicKey[] = ais
    .map((ai, i) => [isSwitchboardOracle(ai!), oracles[i]])
    .filter((r) => r[0])
    .map((r) => r[1]) as PublicKey[];

  for (const o of switcboardOracles) {
    const r = await (
      await buildFetch()
    )('https://stats.switchboard.xyz/logs', {
      headers: {
        accept: '*/*',
        'content-type': 'application/json',
      },
      body: `{"cluster":"solana-mainnet","query":"${o.toString()}","number":100,"severity":"INFO"}`,
      method: 'POST',
    });

    const bOrPm = getBankForOracle(group, o);
    console.log(`${bOrPm.name} ${bOrPm.oracleLastUpdatedSlot} ${o}`);

    (await r.json()).forEach((e: { message: string; timestamp: string }) => {
      if (e.message.toLowerCase().includes('error')) {
        console.log(`${e.timestamp}: ${e.message}`);
      }
    });

    console.log(``);
  }
}

main();
