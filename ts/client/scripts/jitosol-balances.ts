import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../src/client';
import { toUiDecimals } from '../src/utils';

async function run() {
  const client = await MangoClient.connectDefault(process.env.MB_CLUSTER_URL!);
  let group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  let accounts = await client.getAllMangoAccounts(group, true);
  const jitoBank = group.getFirstBankByMint(
    new PublicKey('J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn'),
  );
  const jitoSpotMarket = group.getSerum3MarketByName('JitoSOL/USDC');

  accounts = accounts.filter(
    (a) =>
      a.getTokenBalanceUi(jitoBank) +
        (a.getSerum3Account(jitoSpotMarket.marketIndex)
          ? a
              .getSerum3OoAccount(jitoSpotMarket.marketIndex)
              .baseTokenTotal.toNumber()
          : 0) >
      0.1,
  );

  accounts.sort((a, b) =>
    a.publicKey.toBase58().localeCompare(b.publicKey.toBase58()),
  );

  console.log(`wallet,mango_account,jito_sol_balance_ui`);
  accounts.forEach((a) =>
    console.log(
      `${a.owner},${a.publicKey},${
        a.getTokenBalanceUi(jitoBank) +
        (a.getSerum3Account(jitoSpotMarket.marketIndex)
          ? toUiDecimals(
              a
                .getSerum3OoAccount(jitoSpotMarket.marketIndex)
                .baseTokenTotal.toNumber(),
              jitoBank.mintDecimals,
            )
          : 0)
      }`,
    ),
  );
}

run();
