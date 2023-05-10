import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import axios from 'axios';
import { Table } from 'console-table-printer';
import { format } from 'fast-csv';
import fs from 'fs';
import cloneDeep from 'lodash/cloneDeep';
import fetch from 'node-fetch';
import { Group } from '../src/accounts/group';
import { HealthType, MangoAccount } from '../src/accounts/mangoAccount';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { I80F48, ONE_I80F48, ZERO_I80F48 } from '../src/numbers/I80F48';
import { toUiDecimals, toUiDecimalsForQuote } from '../src/utils';

const { MB_CLUSTER_URL } = process.env;

const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

async function buildClient(): Promise<MangoClient> {
  const clientKeypair = new Keypair();

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const clientWallet = new Wallet(clientKeypair);
  const clientProvider = new AnchorProvider(connection, clientWallet, options);

  return await MangoClient.connect(
    clientProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );
}

async function computePriceImpactOnJup(
  amount: string,
  inputMint: string,
  outputMint: string,
): Promise<{ outAmount: number; priceImpactPct: number }> {
  const url = `https://quote-api.jup.ag/v4/quote?inputMint=${inputMint}&outputMint=${outputMint}&amount=${amount}&swapMode=ExactIn&slippageBps=10000&onlyDirectRoutes=false&asLegacyTransaction=false`;
  const response = await fetch(url);

  try {
    let res = await response.json();
    res = res.data[0];
    return {
      outAmount: parseFloat(res.outAmount),
      priceImpactPct: parseFloat(res.priceImpactPct),
    };
  } catch (e) {
    console.log(url);
    console.log(e);
    throw e;
  }
}

async function computePriceImpactForLiqor(
  group: Group,
  mangoAccounts: MangoAccount[],
  healthThresh: number,
  title: string,
  csvSuffix: string,
): Promise<void> {
  // Filter mango accounts below a certain health ration threshold
  const mangoAccountsWithHealth = mangoAccounts
    .map((a: MangoAccount) => {
      return {
        account: a,
        health: a.getHealth(group, HealthType.liquidationEnd),
        healthRatio: a.getHealthRatioUi(group, HealthType.liquidationEnd),
        liabs: toUiDecimalsForQuote(
          a.getLiabsValue(group, HealthType.liquidationEnd),
        ),
      };
    })
    .filter((a) => a.healthRatio < healthThresh);

  const table = new Table({
    columns: [
      { name: 'Coin', alignment: 'right' },
      { name: 'Oracle Price', alignment: 'right' },
      { name: 'On-Chain Price', alignment: 'right' },
      { name: 'Future Price', alignment: 'right' },
      // { name: 'V4 Soft Limit', alignment: 'right' },
      { name: 'V4 Liq Fee', alignment: 'right' },
      { name: 'Liabs', alignment: 'right' },
      { name: 'Liabs slippage', alignment: 'right' },
      { name: 'Assets Sum', alignment: 'right' },
      { name: 'Assets Slippage', alignment: 'right' },
      // { name: 'Jup Day Volume', alignment: 'right' },
    ],
  });

  const fileName = `/tmp/${
    new Date().toISOString().split('T')[0]
  }-${csvSuffix}-price_impact.csv`;
  const csvFile = fs.createWriteStream(fileName);
  const stream = format({ headers: true });
  stream.pipe(csvFile);
  stream.write([
    'Coin',
    'Oracle Price',
    'On-Chain Price',
    'Future Price',
    // 'V4 Soft Limit',
    'V4 Liq Fee',
    'Liabs',
    'Liabs slippage',
    'Assets Sum',
    'Assets Slippage',
    // 'Jup Day Volume',
  ]);

  // High level solana defi stats
  const response = await fetch('https://cache.jup.ag/stats/day');
  const res = await response.json();

  const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
  const usdcBank = group.getFirstBankByMint(new PublicKey(USDC_MINT));

  // For each token
  for (const banks of Array.from(group.banksMapByMint.values())) {
    // TODO USDC
    const bank = banks[0];

    const onChainPrice = (
      await (
        await fetch(`https://price.jup.ag/v4/price?ids=${bank.mint}`)
      ).json()
    )['data'][bank.mint.toBase58()]['price'];

    if (bank.reduceOnly > 0 || bank.tokenIndex === usdcBank.tokenIndex) {
      continue;
    }

    // Sum of all liabs, these liabs would be acquired by liqor,
    // who would immediately want to reduce them to 0
    // Assuming liabs need to be bought using USDC
    const liabs =
      // Max liab of a particular token that would be liquidated to bring health above 0
      mangoAccountsWithHealth.reduce((sum, a) => {
        // How much would health increase for every unit liab moved to liqor
        // liabprice * (liabweight - (1+fee)*assetweight)
        const tokenLiabHealthContrib = bank.price.mul(
          bank.initLiabWeight.sub(
            ONE_I80F48().add(bank.liquidationFee).mul(usdcBank.initAssetWeight),
          ),
        );
        // Abs liab/borrow
        const maxTokenLiab = a.account
          .getTokenBalance(bank)
          .min(ZERO_I80F48())
          .abs();
        // Health under 0
        const maxLiab = a.health
          .min(ZERO_I80F48())
          .abs()
          .div(tokenLiabHealthContrib)
          .min(maxTokenLiab);

        // DEBUG
        // if (
        //   a.health.toNumber() < -10000000 ||
        //   maxLiab.gt(I80F48.fromNumber(10000))
        // )
        //   console.log(
        //     `https://app.mango.markets/?address=${
        //       a.account.publicKey
        //     }, health ${a.health.toNumber().toLocaleString()}, bank ${
        //       bank.name
        //     }, maxLiab ${maxLiab.toNumber().toLocaleString()}`,
        //   );

        return sum.add(maxLiab);
      }, ZERO_I80F48());
    const liabsInUsdc =
      // convert to usdc, this is an approximation
      liabs
        .mul(bank.price)
        .floor()
        // jup oddity
        .min(I80F48.fromNumber(99999999999));
    const pi1 = !liabsInUsdc.eq(ZERO_I80F48())
      ? await computePriceImpactOnJup(
          liabsInUsdc.toString(),
          USDC_MINT,
          bank.mint.toBase58(),
        )
      : { priceImpactPct: 0, outAmount: 0 };

    // Sum of all assets which would be acquired in exchange for also acquiring
    // liabs by the liqor, who would immediately want to reduce to 0
    // Assuming assets need to be sold to USDC
    const assets = mangoAccountsWithHealth.reduce((sum, a) => {
      // How much would health increase for every unit liab moved to liqor
      // assetprice * (liabweight/(1+liabliqfee) - assetweight)
      const tokenAssetHealthContrib = bank.price.mul(
        Array.from(group.banksMapByTokenIndex.values())
          .flat()
          .map((bank) => bank.initLiabWeight)
          .reduce((prev, curr) => (prev.lt(curr) ? prev : curr))
          .div(ONE_I80F48().add(bank.liquidationFee))
          .sub(bank.initAssetWeight),
      );
      // Abs collateral/asset
      const maxTokenHealthAsset = a.account
        .getTokenBalance(bank)
        .max(ZERO_I80F48());
      const maxAsset = a.health
        .min(ZERO_I80F48())
        .abs()
        .div(tokenAssetHealthContrib)
        .min(maxTokenHealthAsset);

      // DEBUG
      // if (
      //   a.health.toNumber() < -10000000 ||
      //   maxAsset.gt(I80F48.fromNumber(10000))
      // )
      //   console.log(
      //     `https://app.mango.markets/?address=${
      //       a.account.publicKey
      //     }, health ${a.health.toNumber().toLocaleString()}, bank ${
      //       bank.name
      //     }, maxAsset ${maxAsset.toNumber().toLocaleString()}`,
      //   );

      return sum.add(maxAsset);
    }, ZERO_I80F48());

    const pi2 = !assets.eq(ZERO_I80F48())
      ? await computePriceImpactOnJup(
          assets.floor().toString(),
          bank.mint.toBase58(),
          USDC_MINT,
        )
      : { priceImpactPct: 0 };

    table.addRow({
      Coin: bank.name,
      'Oracle Price':
        bank['oldUiPrice'] < 0.1
          ? bank['oldUiPrice']
          : bank['oldUiPrice'].toFixed(2),
      'On-Chain Price':
        onChainPrice < 0.1 ? onChainPrice : onChainPrice.toFixed(2),
      'Future Price':
        bank._uiPrice! < 0.1 ? bank._uiPrice! : bank._uiPrice!.toFixed(2),
      // 'V4 Soft Limit':
      //   toUiDecimalsForQuote(
      //     bank.depositWeightScaleStartQuote,
      //   ).toLocaleString() + '$',
      'V4 Liq Fee': (bank.liquidationFee.toNumber() * 100).toFixed(2) + '%',
      Liabs: toUiDecimalsForQuote(liabsInUsdc).toLocaleString() + '$',
      'Liabs slippage': (pi1.priceImpactPct * 100).toFixed(2) + '%',
      'Assets Sum':
        (
          toUiDecimals(assets, bank.mintDecimals) * bank.uiPrice
        ).toLocaleString() + '$',
      'Assets Slippage': (pi2.priceImpactPct * 100).toFixed(2) + '%',
      // 'Jup Day Volume':
      //   '$' +
      //   parseFloat(
      //     res['lastXTopTokens'].filter(
      //       (entry) => entry.mint === bank.mint.toBase58(),
      //     )[0].amount,
      //   ).toLocaleString(),
    });

    stream.write([
      bank.name,
      bank['oldUiPrice'],
      onChainPrice,
      bank._uiPrice,
      // toUiDecimalsForQuote(bank.depositWeightScaleStartQuote),
      (bank.liquidationFee.toNumber() * 100).toFixed(2),
      toUiDecimalsForQuote(liabsInUsdc).toFixed(2),
      (pi1.priceImpactPct * 100).toFixed(2),
      (toUiDecimals(assets, bank.mintDecimals) * bank.uiPrice).toFixed(2),
      (pi2.priceImpactPct * 100).toFixed(2),
      // parseFloat(
      //   res['lastXTopTokens'].filter(
      //     (entry) => entry.mint === bank.mint.toBase58(),
      //   )[0].amount,
      // ).toFixed(2),
    ]);
  }
  stream.end();
  const msg = title + '\n```\n' + table.render() + '\n```';
  console.log(msg);
  if (process.env.WEBHOOK_URL) {
    axios
      .post(process.env.WEBHOOK_URL, { content: msg })
      .catch((e) => console.log(e.response.data));
  }
  console.log();
}

async function computePerpPositionsToBeLiquidated(
  group: Group,
  mangoAccounts: MangoAccount[],
  healthThresh: number,
  title: string,
  csvSuffix: string,
): Promise<void> {
  const mangoAccountsWithHealth = mangoAccounts
    .map((a: MangoAccount) => {
      return {
        account: a,
        health: a.getHealth(group, HealthType.liquidationEnd),
        healthRatio: a.getHealthRatioUi(group, HealthType.liquidationEnd),
        liabs: toUiDecimalsForQuote(
          a.getLiabsValue(group, HealthType.liquidationEnd),
        ),
      };
    })
    .filter((a) => a.healthRatio < healthThresh);

  const table = new Table({
    columns: [
      { name: 'Market', alignment: 'right' },
      { name: 'Price', alignment: 'right' },
      { name: 'Future Price', alignment: 'right' },
      { name: 'Notional Position', alignment: 'right' },
    ],
  });
  const fileName = `/tmp/${
    new Date().toISOString().split('T')[0]
  }-${csvSuffix}-perp_market.csv`;
  const csvFile = fs.createWriteStream(fileName);
  const stream = format({ headers: true });
  stream.pipe(csvFile);
  stream.write(['Market', 'Price', 'Future Price', 'Notional Position']);

  for (const pm of Array.from(
    group.perpMarketsMapByMarketIndex.values(),
  ).filter((pm) => !pm.name.includes('OLD'))) {
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

        // DEBUG
        // if (
        //   a.health.toNumber() < -10000000 ||
        //   toUiDecimalsForQuote(
        //     maxBaseLots.mul(I80F48.fromU64(pm.baseLotSize).mul(pm.price)),
        //   ) > 100
        // )
        //   console.log(
        //     `https://app.mango.markets/?address=${
        //       a.account.publicKey
        //     }, perp market ${pm.name}, health ${a.health
        //       .toNumber()
        //       .toLocaleString()}, unweightedHealthPerLot ${unweightedHealthPerLot}, maxBaseLots ${toUiDecimalsForQuote(
        //       maxBaseLots.mul(I80F48.fromU64(pm.baseLotSize).mul(pm.price)),
        //     )}`,
        //   );

        return sum.add(maxBaseLots);
      }, ONE_I80F48());

    const notionalPositionUi = toUiDecimalsForQuote(
      baseLots.mul(I80F48.fromU64(pm.baseLotSize).mul(pm.price)),
    );

    table.addRow({
      Market: pm.name,
      Price:
        pm['oldUiPrice'] < 0.1 ? pm['oldUiPrice'] : pm['oldUiPrice'].toFixed(2),
      'Future Price':
        pm._uiPrice! < 0.1 ? pm._uiPrice! : pm._uiPrice!.toFixed(2),
      'Notional Position': notionalPositionUi.toLocaleString() + '$',
    });
    stream.write([
      pm.name,
      pm['oldUiPrice'],
      pm['_uiPrice'],
      notionalPositionUi,
    ]);
  }
  stream.end();
  const msg = title + '\n```\n' + table.render() + '\n```';
  console.log(msg);
  if (process.env.WEBHOOK_URL) {
    axios
      .post(process.env.WEBHOOK_URL, { content: msg })
      .catch((e) => console.log(e.response.data));
  }
  console.log();
}

async function logLiqorEquity(
  client: MangoClient,
  group: Group,
  mangoAccounts: PublicKey[],
  title: string,
): Promise<void> {
  const table = new Table({
    columns: [
      { name: 'Account', alignment: 'right' },
      { name: 'Equity', alignment: 'right' },
    ],
  });

  // Filter mango accounts which might be closed
  const liqors = (
    await client.connection.getMultipleAccountsInfo(mangoAccounts)
  )
    .map((ai, i) => {
      return { ai: ai, pk: mangoAccounts[i] };
    })
    .filter((val) => val.ai)
    .map((val) => val.pk);
  const liqorMangoAccounts = await Promise.all(
    liqors.map((liqor) => client.getMangoAccount(liqor, true)),
  );
  liqorMangoAccounts.forEach((a: MangoAccount) => {
    table.addRow({
      Account: a.publicKey.toBase58(),
      Equity: toUiDecimalsForQuote(a.getEquity(group)).toLocaleString() + '$',
    });
  });
  const msg = title + '\n```\n' + table.render() + '\n```';
  console.log(msg);
  // if (process.env.WEBHOOK_URL) {
  //   axios
  //     .post(process.env.WEBHOOK_URL, { content: msg })
  //     .catch((e) => console.log(e));
  // }
  console.log();
}

async function main(): Promise<void> {
  const client = await buildClient();
  const group = await client.getGroup(new PublicKey(GROUP_PK));
  const mangoAccounts = await client.getAllMangoAccounts(group, true);

  const change = 0.4;

  const drop = 1 - change;
  const groupBear: Group = cloneDeep(group);
  Array.from(groupBear.banksMapByTokenIndex.values())
    .flat()
    .forEach((b) => {
      b['oldUiPrice'] = b._uiPrice;
      b._uiPrice = b._uiPrice! * drop;
      b._price = b._price?.mul(I80F48.fromNumber(drop));
    });
  Array.from(groupBear.perpMarketsMapByMarketIndex.values()).forEach((p) => {
    p['oldUiPrice'] = p._uiPrice;
    p._uiPrice = p._uiPrice! * drop;
    p._price = p._price?.mul(I80F48.fromNumber(drop));
  });

  const rally = 1 + change;
  const groupBull: Group = cloneDeep(group);
  Array.from(groupBull.banksMapByTokenIndex.values())
    .flat()
    .forEach((b) => {
      b['oldUiPrice'] = b._uiPrice;
      b._uiPrice = b._uiPrice! * rally;
      b._price = b._price?.mul(I80F48.fromNumber(rally));
    });
  Array.from(groupBull.perpMarketsMapByMarketIndex.values()).forEach((p) => {
    p['oldUiPrice'] = p._uiPrice;
    p._uiPrice = p._uiPrice! * rally;
    p._price = p._price?.mul(I80F48.fromNumber(rally));
  });

  const healthThresh = 0;

  let tableName = `Liqors acquire liabs and assets. The assets and liabs are sum of max assets and max 
  liabs for any token which would be liquidated to fix the health of a mango account. 
  This would be the slippage they would face on buying-liabs/offloading-assets tokens acquired from unhealth accounts after a`;
  await computePriceImpactForLiqor(
    groupBear,
    mangoAccounts,
    healthThresh,
    `Table 1a: ${tableName} 20% drop`,
    'drop-by-20-pct',
  );
  await computePriceImpactForLiqor(
    groupBull,
    mangoAccounts,
    healthThresh,
    `Table 1b: ${tableName} 20% rally`,
    'rally-by-20-pct',
  );

  tableName = 'Perp notional that liqor need to liquidate after a ';
  await computePerpPositionsToBeLiquidated(
    groupBear,
    mangoAccounts,
    healthThresh,
    `Table 2a: ${tableName} 20% drop`,
    'rally-by-20-pct',
  );
  await computePerpPositionsToBeLiquidated(
    groupBull,
    mangoAccounts,
    healthThresh,
    `Table 2b: ${tableName} 20% rally`,
    'rally-by-20-pct',
  );

  await logLiqorEquity(
    client,
    group,
    (
      await (
        await fetch(
          `https://api.mngo.cloud/data/v4/stats/liqors-over_period?over_period=1MONTH`, // alternative - 1WEEK,
        )
      ).json()
    ).map((data) => new PublicKey(data['liqor'])),
    `Table 3: Equity of known liqors from last month`,
  );

  await logLiqorEquity(
    client,
    group,
    [
      new PublicKey('CtHuPg2ctVVV7nqmvVEcMtcWyJAgtZw9YcNHFQidjPgF'),
      new PublicKey('F1SZxEDxxCSLVjEBbMEjDYqajWRJQRCZBwPQnmcVvTLV'),
      new PublicKey('BGYWnqfaauCeebFQXEfYuDCktiVG8pqpprrsD4qfqL53'),
      new PublicKey('9XJt2tvSZghsMAhWto1VuPBrwXsiimPtsTR8XwGgDxK2'),
    ],
    `Table 4: Equity of known makers from last month`,
  );

  // TODO warning when wrapper asset on chain price has too much difference to oracle
  // TODO warning when slippage is higher than liquidation fee
  // TODO warning when liqors equity is too low
  // TODO warning when mm equity is too low

  // TODO all awaits are linear, should be parallelised to speed up script
}

main();
