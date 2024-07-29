import { PublicKey } from '@solana/web3.js';
import { SB_ON_DEMAND_PID } from '@switchboard-xyz/on-demand';
import { isSwitchboardOracle } from '../src/accounts/oracle';
import { MangoClient } from '../src/client';

async function main(): Promise<void> {
  const client = await MangoClient.connectDefault(process.env.MB_CLUSTER_URL!);
  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const allOracles = Array.from(group.banksMapByName.values())
    .flat()
    .map((b) => [b.name, b.oracle])
    .concat(
      Array.from(group.banksMapByName.values())
        .flat()
        .map((b) => [b.name, b.fallbackOracle])
        .filter(
          (item) =>
            item[1] instanceof PublicKey && !item[1].equals(PublicKey.default),
        ),
    );

  const oraclePublicKeys = allOracles.map((item) => item[1] as PublicKey);
  const ais =
    await client.program.provider.connection.getMultipleAccountsInfo(
      oraclePublicKeys,
    );

  const result = ais
    .map((ai, idx) => {
      return [
        isSwitchboardOracle(ai!) && !ai?.owner.equals(SB_ON_DEMAND_PID),
        allOracles[idx],
      ];
    })
    .filter((item) => item[0])
    .map((item) => item[1]);

  console.log(result);
}

main();
