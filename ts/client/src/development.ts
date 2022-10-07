///
/// debugging
///

import { AccountMeta, PublicKey } from '@solana/web3.js';
import { Bank } from './accounts/bank';
import { Group } from './accounts/group';
import { MangoAccount, Serum3Orders } from './accounts/mangoAccount';
import { PerpMarket } from './accounts/perp';

export function debugAccountMetas(ams: AccountMeta[]): void {
  for (const am of ams) {
    console.log(
      `${am.pubkey.toBase58()}, isSigner: ${am.isSigner
        .toString()
        .padStart(5, ' ')}, isWritable - ${am.isWritable
        .toString()
        .padStart(5, ' ')}`,
    );
  }
}

export function debugHealthAccounts(
  group: Group,
  mangoAccount: MangoAccount,
  publicKeys: PublicKey[],
): void {
  const banks = new Map(
    Array.from(group.banksMapByName.values()).map((banks: Bank[]) => [
      banks[0].publicKey.toBase58(),
      `${banks[0].name} bank`,
    ]),
  );
  const oracles = new Map(
    Array.from(group.banksMapByName.values()).map((banks: Bank[]) => [
      banks[0].oracle.toBase58(),
      `${banks[0].name} oracle`,
    ]),
  );
  const serum3 = new Map(
    mangoAccount.serum3Active().map((serum3: Serum3Orders) => {
      const serum3Market = Array.from(
        group.serum3MarketsMapByExternal.values(),
      ).find((serum3Market) => serum3Market.marketIndex === serum3.marketIndex);
      if (!serum3Market) {
        throw new Error(
          `Serum3Orders for non existent market with market index ${serum3.marketIndex}`,
        );
      }
      return [serum3.openOrders.toBase58(), `${serum3Market.name} spot oo`];
    }),
  );
  const perps = new Map(
    Array.from(group.perpMarketsMapByName.values()).map(
      (perpMarket: PerpMarket) => [
        perpMarket.publicKey.toBase58(),
        `${perpMarket.name} perp market`,
      ],
    ),
  );

  publicKeys.map((pk) => {
    if (banks.get(pk.toBase58())) {
      console.log(banks.get(pk.toBase58()));
    }
    if (oracles.get(pk.toBase58())) {
      console.log(oracles.get(pk.toBase58()));
    }
    if (serum3.get(pk.toBase58())) {
      console.log(serum3.get(pk.toBase58()));
    }
    if (perps.get(pk.toBase58())) {
      console.log(perps.get(pk.toBase58()));
    }
  });
}
