import { PublicKey } from '@solana/web3.js';
import { Group } from './accounts/group';
import { MangoAccount, PerpPosition } from './accounts/mangoAccount';
import { PerpMarket } from './accounts/perp';
import { MangoClient } from './client';
import { I80F48 } from './numbers/I80F48';

/**
 * Returns a list of perp positions alongwith their mango account, sorted descending by notional value
 * @param client
 * @param group
 * @returns
 */
export async function getLargestPerpPositions(
  client: MangoClient,
  group: Group,
  accounts?: MangoAccount[],
  perpMarket?: PerpMarket,
): Promise<{ mangoAccount: PublicKey; perpPosition: PerpPosition }[]> {
  if (!accounts) {
    accounts = await client.getAllMangoAccounts(group, true);
  }

  let allPps = accounts
    .map((a) => {
      const pps = a.perpActive().map((pp) => {
        pp['mangoAccount'] = a.publicKey;
        return pp;
      });
      return pps;
    })
    .flat();

  if (perpMarket) {
    allPps = allPps.filter(
      (pp) => pp.marketIndex == perpMarket?.perpMarketIndex,
    );
  }

  allPps.sort(
    (a, b) =>
      Math.abs(
        b.getNotionalValueUi(group.getPerpMarketByMarketIndex(b.marketIndex)),
      ) -
      Math.abs(
        a.getNotionalValueUi(group.getPerpMarketByMarketIndex(a.marketIndex)),
      ),
  );

  return allPps.map((pp) => ({
    mangoAccount: pp['mangoAccount'],
    perpPosition: pp,
  }));
}

/**
 * Returns a list of perp positions alongwith their mango account, sorted ascending by closest to liquidation
 * @param client
 * @param group
 * @param filterByNotionalValueUi
 * @returns
 */
export async function getClosestToLiquidationPerpPositions(
  client: MangoClient,
  group: Group,
  accounts?: MangoAccount[],
  filterByNotionalValueUi = 10,
): Promise<
  { mangoAccount: PublicKey; perpPosition: PerpPosition; pct: I80F48 }[]
> {
  if (!accounts) {
    accounts = await client.getAllMangoAccounts(group, true);
  }
  const accountsMap = new Map(accounts.map((a) => [a.publicKey.toBase58(), a]));

  let allPps: any = accounts
    .map((a) => {
      const pps = a
        .perpActive()
        .filter(
          (pp) =>
            pp.getNotionalValueUi(
              group.getPerpMarketByMarketIndex(pp.marketIndex),
            ) > filterByNotionalValueUi,
        )
        .map((pp) => {
          pp['mangoAccount'] = a.publicKey;
          return pp;
        });
      return pps;
    })
    .flat();

  function getChangeToLiquidation(pp: PerpPosition): I80F48 {
    const lp = pp.getLiquidationPrice(
      group,
      accountsMap.get(pp['mangoAccount'].toBase58())!,
    )!;
    const op = group.getPerpMarketByMarketIndex(pp.marketIndex).price;
    return lp.sub(op).abs().div(op).mul(I80F48.fromNumber(100));
  }

  allPps = allPps.filter(
    (pp) =>
      pp.getLiquidationPrice(
        group,
        accountsMap.get(pp['mangoAccount'].toBase58())!,
      ) != null,
  );

  return allPps
    .map((pp) => ({
      mangoAccount: pp['mangoAccount'],
      perpPosition: pp,
      pct: getChangeToLiquidation(pp),
    }))
    .sort((a, b) => (a.pct.lte(b.pct) ? -1 : 1));
}
