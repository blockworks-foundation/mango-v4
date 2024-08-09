import { PublicKey } from '@solana/web3.js';
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
    )
    .concat(
      Array.from(group.perpMarketsMapByName.values())
        .flat()
        .map((pm) => [pm.name, pm.oracle]),
    );

  const oraclePublicKeys = allOracles.map((item) => item[1] as PublicKey);

  const ais =
    await client.program.provider.connection.getMultipleAccountsInfo(
      oraclePublicKeys,
    );

  const result = ais
    .map((ai, idx) => {
      return [ai!.data.readUInt32LE(0) === 2712847316, allOracles[idx]];
    })
    .filter((item) => item[0])
    .map((item) => item[1].toString());

  console.log(result);
}

main();
