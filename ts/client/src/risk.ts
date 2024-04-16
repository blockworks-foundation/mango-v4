import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import copy from 'fast-copy';
import { TokenIndex } from './accounts/bank';
import { Group } from './accounts/group';
import { HealthType, MangoAccount } from './accounts/mangoAccount';
import { MangoClient } from './client';
import { I80F48, ONE_I80F48, ZERO_I80F48 } from './numbers/I80F48';
import { buildFetch, toUiDecimals, toUiDecimalsForQuote } from './utils';

export interface LiqorPriceImpact {
  Coin: { val: string; highlight: boolean };
  'Oracle Price': { val: number; highlight: boolean };
  'Jup Price': { val: number; highlight: boolean };
  'Future Price': { val: number; highlight: boolean };
  'V4 Liq Fee': { val: number; highlight: boolean };
  Liabs: { val: number; highlight: boolean };
  'Liabs Slippage': { val: number; highlight: boolean };
  Assets: { val: number; highlight: boolean };
  'Assets Slippage': { val: number; highlight: boolean };
}

export interface PerpPositionsToBeLiquidated {
  Market: { val: string; highlight: boolean };
  Price: { val: number; highlight: boolean };
  'Future Price': { val: number; highlight: boolean };
  'Notional Position': { val: number; highlight: boolean };
}

export interface AccountEquity {
  Account: { val: PublicKey; highlight: boolean };
  Equity: { val: number; highlight: boolean };
}

export interface LiquidationBatches {
  start: number;
  startAssets: number;
  prev: number;
  prevAssets: number;
  largestBatchQuoteUi: number;
}

export interface Risk {
  assetRally: { title: string; data: LiqorPriceImpact[] };
  assetDrop: { title: string; data: LiqorPriceImpact[] };
  usdcDepeg: { title: string; data: LiqorPriceImpact[] };
  usdtDepeg: { title: string; data: LiqorPriceImpact[] };
  perpRally: { title: string; data: PerpPositionsToBeLiquidated[] };
  perpDrop: { title: string; data: PerpPositionsToBeLiquidated[] };
  marketMakerEquity: { title: string; data: AccountEquity[] };
  liqorEquity: { title: string; data: AccountEquity[] };
}

export type PriceImpact = {
  symbol: string;
  side: 'bid' | 'ask';
  target_amount: number;
  avg_price_impact_percent: number;
  min_price_impact_percent: number;
  max_price_impact_percent: number;
  p90: number;
  p95: number;
};

/**
 * Returns price impact in bps i.e. 0 to 10,000
 * returns -1 if data is missing
 */
export function computePriceImpactOnJup(
  pis: PriceImpact[],
  usdcAmount: number,
  tokenName: string,
): number {
  try {
    const closestTo = [
      1_000, 5_000, 20_000, 100_000, 250_000, 500_000, 1_000_000, 5_000_000,
    ].reduce((prev, curr) =>
      Math.abs(curr - usdcAmount) < Math.abs(prev - usdcAmount) ? curr : prev,
    );
    // Workaround api
    if (tokenName == 'ETH (Portal)') {
      tokenName = 'ETH';
    }
    const filteredPis: PriceImpact[] = pis.filter(
      (pi) => pi.symbol == tokenName && pi.target_amount == closestTo,
    );
    if (filteredPis.length > 0) {
      return (filteredPis[0].p90 * 10000) / 100;
    } else {
      return -1;
    }
  } catch (e) {
    return -1;
  }
}

export async function getOnChainPriceForMints(
  mints: string[],
): Promise<number[]> {
  return await Promise.all(
    mints.map(async (mint) => {
      const resp = await (
        await buildFetch()
      )(`https://public-api.birdeye.so/public/price?address=${mint}`, {
        headers: {
          'Content-Type': 'application/json',
        },
      });

      const data = await resp.json();
      return data?.data?.value;
    }),
  );
}

export async function getPriceImpactForLiqor(
  group: Group,
  pis: PriceImpact[],
  mangoAccounts: MangoAccount[],
): Promise<LiqorPriceImpact[]> {
  const mangoAccounts_ = mangoAccounts.filter((a) =>
    a.getHealth(group, HealthType.maint).lt(ZERO_I80F48()),
  );

  const mangoAccountsWithHealth = mangoAccounts_.map((a: MangoAccount) => {
    return {
      account: a,
      health: a.getHealth(group, HealthType.liquidationEnd),
      healthRatio: a.getHealthRatioUi(group, HealthType.liquidationEnd),
    };
  });

  const usdcBank = group.getFirstBankByTokenIndex(0 as TokenIndex);
  const usdcMint = usdcBank.mint;

  return await Promise.all(
    Array.from(group.banksMapByMint.values())
      .sort((a, b) => a[0].name.localeCompare(b[0].name))
      // .filter((banks) => banks[0].name == 'MSOL')
      .map(async (banks) => {
        const bank = banks[0];

        // Sum of all liabs, these liabs would be acquired by liqor,
        // who would immediately want to reduce them to 0
        // Assuming liabs need to be bought using USDC
        const liabs =
          // Max liab of a particular token that would be liquidated to bring health above 0
          mangoAccountsWithHealth.reduce((sum, a) => {
            // How much would health increase for every unit liab moved to liqor
            // liabprice * (liabweight - (1+liabfees)*(1+assetfees)*assetweight)
            // Choose the most valuable asset the user has
            const assetBank = Array.from(group.banksMapByTokenIndex.values())
              .flat()
              .reduce((prev, curr) =>
                prev.initAssetWeight
                  .mul(a.account.getEffectiveTokenBalance(group, prev))
                  .mul(prev._price!)
                  .gt(
                    curr.initAssetWeight.mul(
                      a.account
                        .getEffectiveTokenBalance(group, curr)
                        .mul(curr._price!),
                    ),
                  )
                  ? prev
                  : curr,
              );
            const feeFactor = ONE_I80F48()
              .add(bank.liquidationFee)
              .add(bank.platformLiquidationFee)
              .mul(
                ONE_I80F48()
                  .add(assetBank.liquidationFee)
                  .add(assetBank.platformLiquidationFee),
              );
            const tokenLiabHealthContrib = bank.price.mul(
              bank.initLiabWeight.sub(feeFactor.mul(assetBank.initAssetWeight)),
            );
            // Abs liab/borrow
            const maxTokenLiab = a.account
              .getEffectiveTokenBalance(group, bank)
              .min(ZERO_I80F48())
              .abs();

            if (tokenLiabHealthContrib.eq(ZERO_I80F48())) {
              return sum.add(maxTokenLiab);
            }

            // Health under 0
            const maxLiab = a.health
              .min(ZERO_I80F48())
              .abs()
              .div(tokenLiabHealthContrib)
              .min(maxTokenLiab);

            return sum.add(maxLiab);
          }, ZERO_I80F48());
        const liabsInUsdc =
          // convert to usdc, this is an approximation
          liabs
            .mul(bank.price)
            .floor()
            // jup oddity
            .min(I80F48.fromNumber(99999999999));

        // Sum of all assets which would be acquired in exchange for also acquiring
        // liabs by the liqor, who would immediately want to reduce to 0
        // Assuming assets need to be sold to USDC
        const assets = mangoAccountsWithHealth.reduce((sum, a) => {
          // How much would health increase for every unit liab moved to liqor
          // assetprice * (liabweight/(1+liabliqfee) - assetweight)
          // Choose the smallest liability the user has
          const liabBank = Array.from(group.banksMapByTokenIndex.values())
            .flat()
            .reduce((prev, curr) =>
              prev.initLiabWeight
                .mul(a.account.getEffectiveTokenBalance(group, prev))
                .mul(prev._price!)
                .lt(
                  curr.initLiabWeight.mul(
                    a.account
                      .getEffectiveTokenBalance(group, curr)
                      .mul(curr._price!),
                  ),
                )
                ? prev
                : curr,
            );
          const tokenAssetHealthContrib = bank.price.mul(
            liabBank.initLiabWeight
              .div(ONE_I80F48().add(liabBank.liquidationFee))
              .sub(bank.initAssetWeight),
          );

          // Abs collateral/asset
          const maxTokenHealthAsset = a.account
            .getEffectiveTokenBalance(group, bank)
            .max(ZERO_I80F48());

          if (tokenAssetHealthContrib.eq(ZERO_I80F48())) {
            return sum.add(maxTokenHealthAsset);
          }

          const maxAsset = a.health
            .min(ZERO_I80F48())
            .abs()
            .div(tokenAssetHealthContrib)
            .min(maxTokenHealthAsset);

          return sum.add(maxAsset);
        }, ZERO_I80F48());

        const pi1 =
          !liabsInUsdc.eq(ZERO_I80F48()) &&
          usdcMint.toBase58() !== bank.mint.toBase58()
            ? computePriceImpactOnJup(
                pis,
                toUiDecimalsForQuote(liabsInUsdc),
                bank.name,
              )
            : 0;
        const pi2 =
          !assets.eq(ZERO_I80F48()) &&
          usdcMint.toBase58() !== bank.mint.toBase58()
            ? computePriceImpactOnJup(
                pis,
                toUiDecimals(assets.mul(bank.price), bank.mintDecimals),
                bank.name,
              )
            : 0;

        return {
          Coin: { val: bank.name, highlight: false },
          'Oracle Price': {
            val: bank['oldUiPrice'] ? bank['oldUiPrice'] : bank._uiPrice!,
            highlight: false,
          },
          'Jup Price': {
            val: bank['onChainPrice'],
            highlight:
              Math.abs(
                (bank['onChainPrice'] -
                  (bank['oldUiPrice'] ? bank['oldUiPrice'] : bank._uiPrice!)) /
                  (bank['oldUiPrice'] ? bank['oldUiPrice'] : bank._uiPrice!),
              ) > 0.05,
          },
          'Future Price': { val: bank._uiPrice!, highlight: false },
          'V4 Liq Fee': {
            val: Math.round(bank.liquidationFee.toNumber() * 10000),
            highlight: false,
          },
          Liabs: {
            val: Math.round(toUiDecimalsForQuote(liabsInUsdc)),
            highlight: Math.round(toUiDecimalsForQuote(liabsInUsdc)) > 5000,
          },
          'Liabs Slippage': {
            val: Math.round(pi1),
            highlight:
              Math.round(pi1) >
              Math.round(bank.liquidationFee.toNumber() * 10000),
          },
          Assets: {
            val: Math.round(
              toUiDecimals(assets, bank.mintDecimals) * bank.uiPrice,
            ),
            highlight:
              Math.round(
                toUiDecimals(assets, bank.mintDecimals) * bank.uiPrice,
              ) > 5000,
          },
          'Assets Slippage': {
            val: Math.round(pi2),
            highlight:
              Math.round(pi2) >
              Math.round(bank.liquidationFee.toNumber() * 10000),
          },
        };
      }),
  );
}

export async function getPerpPositionsToBeLiquidated(
  group: Group,
  mangoAccounts: MangoAccount[],
): Promise<PerpPositionsToBeLiquidated[]> {
  const mangoAccountsWithHealth = mangoAccounts.map((a: MangoAccount) => {
    return {
      account: a,
      health: a.getHealth(group, HealthType.liquidationEnd),
      healthRatio: a.getHealthRatioUi(group, HealthType.liquidationEnd),
    };
  });

  return Array.from(group.perpMarketsMapByMarketIndex.values())
    .filter((pm) => !pm.name.includes('OLD'))
    .map((pm) => {
      const baseLots = mangoAccountsWithHealth
        .filter((a) => a.account.getPerpPosition(pm.perpMarketIndex))
        .reduce((sum, a) => {
          const baseLots = a.account.getPerpPosition(
            pm.perpMarketIndex,
          )!.basePositionLots;
          const unweightedHealthPerLot = baseLots.gt(new BN(0))
            ? I80F48.fromNumber(-1)
                .mul(pm.price)
                .mul(I80F48.fromU64(pm.baseLotSize))
                .mul(pm.initBaseAssetWeight)
                .add(
                  I80F48.fromU64(pm.baseLotSize)
                    .mul(pm.price)
                    .mul(
                      ONE_I80F48() // quoteInitAssetWeight
                        .mul(ONE_I80F48().sub(pm.baseLiquidationFee)),
                    ),
                )
            : pm.price
                .mul(I80F48.fromU64(pm.baseLotSize))
                .mul(pm.initBaseLiabWeight)
                .sub(
                  I80F48.fromU64(pm.baseLotSize)
                    .mul(pm.price)
                    .mul(ONE_I80F48()) // quoteInitLiabWeight
                    .mul(ONE_I80F48().add(pm.baseLiquidationFee)),
                );

          const maxBaseLots = a.health
            .min(ZERO_I80F48())
            .abs()
            .div(unweightedHealthPerLot.abs())
            .min(I80F48.fromU64(baseLots).abs());

          return sum.add(maxBaseLots);
        }, ONE_I80F48());

      const notionalPositionUi = toUiDecimalsForQuote(
        baseLots.mul(I80F48.fromU64(pm.baseLotSize).mul(pm.price)),
      );

      return {
        Market: { val: pm.name, highlight: false },
        Price: { val: pm['oldUiPrice'], highlight: false },
        'Future Price': { val: pm._uiPrice, highlight: false },
        'Notional Position': {
          val: Math.round(notionalPositionUi),
          highlight: Math.round(notionalPositionUi) > 5000,
        },
      };
    });
}

export async function getEquityForMangoAccounts(
  client: MangoClient,
  group: Group,
  mangoAccountPks: PublicKey[],
  allMangoAccounts: MangoAccount[],
): Promise<AccountEquity[]> {
  const mangoAccounts = allMangoAccounts.filter((a) =>
    mangoAccountPks.find((pk) => pk.equals(a.publicKey)),
  );

  const accountsWithEquity = mangoAccounts.map((a: MangoAccount) => {
    return {
      Account: { val: a.publicKey, highlight: false },
      Equity: {
        val: Math.round(toUiDecimalsForQuote(a.getEquity(group))),
        highlight: false,
      },
    };
  });
  accountsWithEquity.sort((a, b) => b.Equity.val - a.Equity.val);
  return accountsWithEquity;
}

export function findLargestAssetBatchUi(
  pisForLiqor: LiqorPriceImpact[][],
  coin: string,
  startFromChange = 99,
  maxChange = 1,
  stepSize = 1,
): [number, number, LiquidationBatches[]] {
  let start = startFromChange;
  let largestBatchUi = 0;
  let largestBatchQuoteUi = 0;

  console.log(`___`);
  console.log(
    `${'start'.padStart(3)}: ${'liq$'.padStart(10)}, ${`prev`.padStart(
      3,
    )}: ${'liq'.padStart(10)}, ${'largestBatchUi $'.padStart(15)}`,
  );

  const liquidationBatches: LiquidationBatches[] = [];

  while (start > 0) {
    const piForLiqor = pisForLiqor[start].filter(
      (pi) => pi.Coin.val == coin,
    )[0];

    // Compare entry to another entry, with max change difference
    const prev = Math.min(99, start + Math.round(start / maxChange));
    const prevPiForLiqor = pisForLiqor[prev].filter(
      (pi) => pi.Coin.val == coin,
    )[0];

    // Note: Assets.val is asset in $ amount that would need to be liquidated when price drops to a certain point
    const changeQuoteUi = piForLiqor.Assets.val - prevPiForLiqor.Assets.val;
    const changeUi =
      piForLiqor.Assets.val / piForLiqor['Future Price'].val -
      prevPiForLiqor.Assets.val / prevPiForLiqor['Future Price'].val;

    console.log(
      `${start.toString().padStart(3)}: ${piForLiqor.Assets.val
        .toLocaleString()
        .padStart(10)}, ${prev
        .toString()
        .padStart(3)}: ${prevPiForLiqor.Assets.val
        .toLocaleString()
        .padStart(10)}, ${largestBatchQuoteUi.toLocaleString().padStart(15)}`,
    );

    liquidationBatches.push({
      start: start,
      startAssets: piForLiqor.Assets.val,
      prev: prev,
      prevAssets: prevPiForLiqor.Assets.val,
      largestBatchQuoteUi: largestBatchQuoteUi,
    });

    if (changeQuoteUi > largestBatchQuoteUi) {
      largestBatchUi = changeUi;
      largestBatchQuoteUi = changeQuoteUi;
    }

    start -= stepSize;
  }
  console.log(`___`);

  return [largestBatchQuoteUi, largestBatchUi, liquidationBatches];
}

export async function buildGroupGrid(
  group: Group,
  allMangoAccounts,
  stepSize = 1,
): Promise<LiqorPriceImpact[][]> {
  const mangoAccountsSubset = allMangoAccounts.filter(
    (a) => toUiDecimalsForQuote(a.getEquity(group)) > 100,
  );

  {
    let pis;
    try {
      pis = await (
        await (
          await buildFetch()
        )(
          `https://api.mngo.cloud/data/v4/risk/listed-tokens-one-week-price-impacts`,
          {
            mode: 'cors',
            headers: {
              'Content-Type': 'application/json',
              'Access-Control-Allow-Origin': '*',
            },
          },
        )
      ).json();
    } catch (error) {
      pis = [];
    }

    // Build groups where price has changed from 0 to -99% for each (non stable coin) assets simultaneously
    const groups = new Array(200);
    let change = stepSize;
    while (change < 101) {
      groups[change] = copy(group);
      const groupToModify: Group = groups[change.toString()];
      const change_ = change / 100;
      Array.from(groupToModify.banksMapByTokenIndex.values())
        .flat()
        .filter((b) => !b.name.includes('USD'))
        .forEach((b) => {
          b['oldUiPrice'] = b._uiPrice;
          b._uiPrice = b._uiPrice! * change_;
          b._price = b._price?.mul(I80F48.fromNumber(change_));
        });
      Array.from(groupToModify.perpMarketsMapByMarketIndex.values()).forEach(
        (p) => {
          p['oldUiPrice'] = p._uiPrice;
          p._uiPrice = p._uiPrice! * change_;
          p._price = p._price?.mul(I80F48.fromNumber(change_));
        },
      );
      change += stepSize;
    }

    // Compute how much of an asset would need to be liquidated
    // when group (i.e. asset prices) reach a specific state
    return await Promise.all(
      groups.map((g) => getPriceImpactForLiqor(g, pis, mangoAccountsSubset)),
    );
  }
}

export async function getLiquidationBatches(
  client: MangoClient,
  group: Group,
): Promise<{ token: TokenIndex; liquidationPoints: LiquidationBatches[] }[]> {
  const allMangoAccounts = await client.getAllMangoAccounts(group, true);
  const stepSize = 1;
  const pisForLiqor: LiqorPriceImpact[][] = await buildGroupGrid(
    group,
    allMangoAccounts,
    stepSize,
  );

  const r = Array.from(group.banksMapByTokenIndex.values())
    .map((banks) => banks[0])
    .map((bank) => {
      return {
        token: bank.tokenIndex,
        liquidationPoints: findLargestAssetBatchUi(
          pisForLiqor,
          bank.name,
          Math.round(bank.maintAssetWeight.toNumber() * 100),
          100 - Math.round(bank.maintAssetWeight.toNumber() * 100),
          stepSize,
        )[2],
      };
    });
  return r;
}

export async function getRiskStats(
  client: MangoClient,
  group: Group,
  change = 0.4, // simulates 40% price rally and price drop on tokens and markets
): Promise<Risk> {
  let pis;
  try {
    pis = await (
      await (
        await buildFetch()
      )(
        `https://api.mngo.cloud/data/v4/risk/listed-tokens-one-week-price-impacts`,
        {
          mode: 'cors',
          headers: {
            'Content-Type': 'application/json',
            'Access-Control-Allow-Origin': '*',
          },
        },
      )
    ).json();
  } catch (error) {
    pis = [];
  }

  // Get known liqors
  let liqors: PublicKey[];
  try {
    liqors = (
      await (
        await (
          await buildFetch()
        )(
          `https://api.mngo.cloud/data/v4/stats/liqors-over_period?over_period=1MONTH`,
          {
            mode: 'cors',
            headers: {
              'Content-Type': 'application/json',
              'Access-Control-Allow-Origin': '*',
            },
          },
        )
      ).json()
    ).map((data) => new PublicKey(data['liqor']));
  } catch (error) {
    liqors = [new PublicKey('2T1taZuQwy7izxvjbAeiVjDhCEFYjWhLuv4U36XD1rL1')];
    liqors = [new PublicKey('Dr1wLHRKQSir4UgGphZ29ZcPhGvJrdDnrLTDgbz73bDs')];
    liqors = [new PublicKey('BNTDZJQrjNkjFxYAMCdKH2ShSM6Uwc28aAgit7ytVQJc')];
    liqors = [new PublicKey('BmAXMP5yUeagSsJa1PPMNyzRR5x2J4nJPrNrUGryCsnD')];
  }

  // Get known mms
  let mms: PublicKey[];
  try {
    mms = (
      await (
        await (
          await buildFetch()
        )(
          `https://api.mngo.cloud/data/v4/stats/perp-makers-over_period?over-period=1WEEK`,
          {
            mode: 'cors',
            headers: {
              'Content-Type': 'application/json',
              'Access-Control-Allow-Origin': '*',
            },
          },
        )
      ).json()
    ).map((data) => new PublicKey(data['liqor']));
  } catch (error) {
    mms = [
      new PublicKey('BLgb4NFwhpurMrGX5LQfb8D8dBpGSGtBqqew2Em8uyRT'),
      new PublicKey('4hXPGTmR6dKNNqjLYdfDRSrTaa1Wt2GZoZnQ9hAJEeev'),
      new PublicKey('BGYWnqfaauCeebFQXEfYuDCktiVG8pqpprrsD4qfqL53'),
      new PublicKey('F1SZxEDxxCSLVjEBbMEjDYqajWRJQRCZBwPQnmcVvTLV'),
    ];
  }

  // Get all mango accounts
  const mangoAccounts = await client.getAllMangoAccounts(group, true);

  // Get on chain prices
  const mints = [
    ...new Set(
      Array.from(group.banksMapByTokenIndex.values())
        .flat()
        .map((bank) => bank.mint.toString()),
    ),
  ];

  // Note:
  // Disable for now
  // Getting rate limited
  // const prices = await getOnChainPriceForMints([
  //   ...new Set(
  //     Array.from(group.banksMapByTokenIndex.values())
  //       .flat()
  //       .map((bank) => bank.mint.toString()),
  //   ),
  // ]);
  // const onChainPrices = Object.fromEntries(
  //   prices.map((price, i) => [mints[i], price]),
  // );

  Array.from(group.banksMapByTokenIndex.values())
    .flat()
    .forEach((b) => {
      b['onChainPrice'] = b.uiPrice;
    });

  // Clone group, and simulate change % price drop for all assets except stables
  const drop = 1 - change;
  const groupDrop: Group = copy(group);
  Array.from(groupDrop.banksMapByTokenIndex.values())
    .flat()
    .filter((b) => !b.name.includes('USD'))
    .forEach((b) => {
      b['oldUiPrice'] = b._uiPrice;
      b._uiPrice = b._uiPrice! * drop;
      b._price = b._price?.mul(I80F48.fromNumber(drop));
    });
  Array.from(groupDrop.perpMarketsMapByMarketIndex.values()).forEach((p) => {
    p['oldUiPrice'] = p._uiPrice;
    p._uiPrice = p._uiPrice! * drop;
    p._price = p._price?.mul(I80F48.fromNumber(drop));
  });

  // Clone group, and simulate change % price drop for usdc
  const groupUsdcDepeg: Group = copy(group);
  Array.from(groupUsdcDepeg.banksMapByTokenIndex.values())
    .flat()
    .filter((b) => b.name.includes('USDC'))
    .forEach((b) => {
      b['oldUiPrice'] = b._uiPrice;
      b._uiPrice = b._uiPrice! * drop;
      b._price = b._price?.mul(I80F48.fromNumber(drop));
    });

  // Clone group, and simulate change % price drop for usdt
  const groupUsdtDepeg: Group = copy(group);
  Array.from(groupUsdtDepeg.banksMapByTokenIndex.values())
    .flat()
    .filter((b) => b.name.includes('USDT'))
    .forEach((b) => {
      b['oldUiPrice'] = b._uiPrice;
      b._uiPrice = b._uiPrice! * drop;
      b._price = b._price?.mul(I80F48.fromNumber(drop));
    });

  // Clone group, and simulate change % price rally for all assets except stables
  const rally = 1 + change;
  const groupRally: Group = copy(group);
  Array.from(groupRally.banksMapByTokenIndex.values())
    .flat()
    .filter((b) => !b.name.includes('USD'))
    .forEach((b) => {
      b['oldUiPrice'] = b._uiPrice;
      b._uiPrice = b._uiPrice! * rally;
      b._price = b._price?.mul(I80F48.fromNumber(rally));
    });
  Array.from(groupRally.perpMarketsMapByMarketIndex.values()).forEach((p) => {
    p['oldUiPrice'] = p._uiPrice;
    p._uiPrice = p._uiPrice! * rally;
    p._price = p._price?.mul(I80F48.fromNumber(rally));
  });

  const [
    assetDrop,
    assetRally,
    usdcDepeg,
    usdtDepeg,
    perpDrop,
    perpRally,
    liqorEquity,
    marketMakerEquity,
  ] = await Promise.all([
    getPriceImpactForLiqor(groupDrop, pis, mangoAccounts),
    getPriceImpactForLiqor(groupRally, pis, mangoAccounts),
    getPriceImpactForLiqor(groupUsdcDepeg, pis, mangoAccounts),
    getPriceImpactForLiqor(groupUsdtDepeg, pis, mangoAccounts),
    getPerpPositionsToBeLiquidated(groupDrop, mangoAccounts),
    getPerpPositionsToBeLiquidated(groupRally, mangoAccounts),
    getEquityForMangoAccounts(client, group, liqors, mangoAccounts),
    getEquityForMangoAccounts(client, group, mms, mangoAccounts),
  ]);

  return {
    assetDrop: {
      title: `Table 1a: Liqors acquire liabs and assets. The assets and liabs are sum of max assets and max
    liabs for any token which would be liquidated to fix the health of a mango account.
    This would be the slippage they would face on buying-liabs/offloading-assets tokens acquired from unhealth accounts after a 40% drop to all non-stable oracles`,
      data: assetDrop,
    },
    assetRally: {
      title: `Table 1b: ... same as above but with a 40% rally to all non-stable oracles instead of drop`,
      data: assetRally,
    },
    usdcDepeg: {
      title: `Table 1c: ... same as above but with a 40% drop to only usdc oracle`,
      data: usdcDepeg,
    },
    usdtDepeg: {
      title: `Table 1d: ... same as above but with a 40% drop to only usdt oracle`,
      data: usdtDepeg,
    },
    perpDrop: {
      title: `Table 2a: Perp notional that liqor need to liquidate after a  40% drop`,
      data: perpDrop,
    },
    perpRally: {
      title: `Table 2b: Perp notional that liqor need to liquidate after a  40% rally`,
      data: perpRally,
    },
    liqorEquity: {
      title: `Table 3: Equity of known liqors from last month`,
      data: liqorEquity,
    },
    marketMakerEquity: {
      title: `Table 4: Equity of known makers from last month`,
      data: marketMakerEquity,
    },
  };
}
