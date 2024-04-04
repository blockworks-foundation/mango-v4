import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import copy from 'fast-copy';
import { cpuUsage } from 'process';
import { Group } from '../../src/accounts/group';
import { HealthCache } from '../../src/accounts/healthCache';
import { HealthType, MangoAccount } from '../../src/accounts/mangoAccount';
import { PerpMarket } from '../../src/accounts/perp';
import { Serum3Market } from '../../src/accounts/serum3';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { ZERO_I80F48 } from '../../src/numbers/I80F48';
import { toUiDecimalsForQuote } from '../../src/utils';

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = new PublicKey(
  process.env.MANGO_ACCOUNT_PK || PublicKey.default.toBase58(),
);
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

async function debugUser(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
): Promise<void> {
  console.log(mangoAccount.toString(group));

  await mangoAccount.reload(client);

  console.log(
    'mangoAccount.getEquity() ' +
      toUiDecimalsForQuote(mangoAccount.getEquity(group)!.toNumber()),
  );
  console.log(
    'mangoAccount.getHealth(HealthType.init) ' +
      toUiDecimalsForQuote(
        mangoAccount.getHealth(group, HealthType.init)!.toNumber(),
      ),
  );
  console.log(
    'HealthCache.fromMangoAccount(group,mangoAccount).health(HealthType.init) ' +
      toUiDecimalsForQuote(
        HealthCache.fromMangoAccount(group, mangoAccount)
          .health(HealthType.init)
          .toNumber(),
      ),
  );
  console.log(
    'mangoAccount.getHealthRatio(HealthType.init) ' +
      mangoAccount.getHealthRatio(group, HealthType.init)!.toNumber(),
  );
  console.log(
    'mangoAccount.getHealthRatioUi(HealthType.init) ' +
      mangoAccount.getHealthRatioUi(group, HealthType.init),
  );
  console.log(
    'mangoAccount.getHealthRatio(HealthType.maint) ' +
      mangoAccount.getHealthRatio(group, HealthType.maint)!.toNumber(),
  );
  console.log(
    'mangoAccount.getHealthRatioUi(HealthType.maint) ' +
      mangoAccount.getHealthRatioUi(group, HealthType.maint),
  );
  console.log(
    'mangoAccount.getCollateralValue() ' +
      toUiDecimalsForQuote(mangoAccount.getCollateralValue(group)!.toNumber()),
  );
  console.log(
    'mangoAccount.getAssetsValue() ' +
      toUiDecimalsForQuote(mangoAccount.getAssetsValue(group)!.toNumber()),
  );
  console.log(
    'mangoAccount.getLiabsValue() ' +
      toUiDecimalsForQuote(mangoAccount.getLiabsValue(group)!.toNumber()),
  );

  async function getMaxWithdrawWithBorrowForTokenUiWrapper(
    token,
  ): Promise<void> {
    console.log(
      `mangoAccount.getMaxWithdrawWithBorrowForTokenUi(group, ${token}) ` +
        mangoAccount.getMaxWithdrawWithBorrowForTokenUi(
          group,
          group.banksMapByName.get(token)![0].mint,
        ),
    );
  }
  for (const srcToken of Array.from(group.banksMapByName.keys())) {
    await getMaxWithdrawWithBorrowForTokenUiWrapper(srcToken);
  }

  function getMaxSourceForTokenSwapWrapper(src, tgt): void {
    // Turn on for debugging specific pairs
    // if (src != 'USDC' || tgt != 'MNGO') return;

    let maxSourceUi;
    try {
      maxSourceUi = mangoAccount.getMaxSourceUiForTokenSwap(
        group,
        group.banksMapByName.get(src)![0].mint,
        group.banksMapByName.get(tgt)![0].mint,
      );
    } catch (error) {
      console.log(`Error for ${src}->${tgt}, ` + error.toString());
    }

    const maxTargetUi =
      maxSourceUi *
      (group.banksMapByName.get(src)![0].uiPrice /
        group.banksMapByName.get(tgt)![0].uiPrice);

    const sim = mangoAccount.simHealthRatioWithTokenPositionUiChanges(group, [
      {
        mintPk: group.banksMapByName.get(src)![0].mint,
        uiTokenAmount: -maxSourceUi,
      },
      {
        mintPk: group.banksMapByName.get(tgt)![0].mint,
        uiTokenAmount: maxTargetUi,
      },
    ]);
    console.log(
      `getMaxSourceForTokenSwap ${src.padEnd(4)} ${tgt.padEnd(4)} ` +
        maxSourceUi.toFixed(3).padStart(10) +
        `, health ratio after (${sim.toFixed(3).padStart(10)})`,
    );
  }
  for (const srcToken of Array.from(group.banksMapByName.keys()).sort()) {
    for (const tgtToken of Array.from(group.banksMapByName.keys()).sort()) {
      getMaxSourceForTokenSwapWrapper(srcToken, tgtToken);
    }
  }

  function getMaxForPerpWrapper(perpMarket: PerpMarket): void {
    const maxQuoteUi = mangoAccount.getMaxQuoteForPerpBidUi(
      group,
      perpMarket.perpMarketIndex,
    );
    const simMaxQuote = mangoAccount.simHealthRatioWithPerpBidUiChanges(
      group,
      perpMarket.perpMarketIndex,
      maxQuoteUi / perpMarket.uiPrice,
    );
    const maxBaseUi = mangoAccount.getMaxBaseForPerpAskUi(
      group,
      perpMarket.perpMarketIndex,
    );
    const simMaxBase = mangoAccount.simHealthRatioWithPerpAskUiChanges(
      group,
      perpMarket.perpMarketIndex,
      maxBaseUi,
    );
    console.log(
      `getMaxPerp ${perpMarket.name.padStart(
        10,
      )} getMaxQuoteForPerpBidUi ${maxQuoteUi
        .toFixed(3)
        .padStart(10)} health ratio after (${simMaxQuote
        .toFixed(3)
        .padStart(10)}), getMaxBaseForPerpAskUi ${maxBaseUi
        .toFixed(3)
        .padStart(10)} health ratio after (${simMaxBase
        .toFixed(3)
        .padStart(10)})`,
    );
  }
  for (const perpMarket of Array.from(
    group.perpMarketsMapByMarketIndex.values(),
  )) {
    getMaxForPerpWrapper(perpMarket);
  }

  function getMaxForSerum3Wrapper(serum3Market: Serum3Market): void {
    console.log(
      `getMaxQuoteForSerum3BidUi ${serum3Market.name} ` +
        mangoAccount.getMaxQuoteForSerum3BidUi(
          group,
          serum3Market.serumMarketExternal,
        ),
    );
    console.log(
      `- simHealthRatioWithSerum3BidUiChanges  ${serum3Market.name} ` +
        mangoAccount.simHealthRatioWithSerum3BidUiChanges(
          group,
          mangoAccount.getMaxQuoteForSerum3BidUi(
            group,
            serum3Market.serumMarketExternal,
          ),
          serum3Market.serumMarketExternal,
          HealthType.init,
        ),
    );
    console.log(
      `getMaxBaseForSerum3AskUi ${serum3Market.name} ` +
        mangoAccount.getMaxBaseForSerum3AskUi(
          group,
          serum3Market.serumMarketExternal,
        ),
    );
    console.log(
      `- simHealthRatioWithSerum3BidUiChanges  ${serum3Market.name} ` +
        mangoAccount.simHealthRatioWithSerum3AskUiChanges(
          group,
          mangoAccount.getMaxBaseForSerum3AskUi(
            group,
            serum3Market.serumMarketExternal,
          ),
          serum3Market.serumMarketExternal,
          HealthType.init,
        ),
    );
  }
  for (const serum3Market of Array.from(
    group.serum3MarketsMapByExternal.values(),
  )) {
    getMaxForSerum3Wrapper(serum3Market);
  }

  // Liquidation price for perp positions
  for (const pp of mangoAccount.perpActive()) {
    const pm = group.getPerpMarketByMarketIndex(pp.marketIndex);
    const health = toUiDecimalsForQuote(
      mangoAccount.getHealth(group, HealthType.maint),
    );

    if (
      // pp.getNotionalValueUi(pm) > 1000 &&
      // !(pp.getNotionalValueUi(pm) < health && pp.getBasePosition(pm).isPos())
      // eslint-disable-next-line no-constant-condition
      true
    ) {
      const then = Date.now();
      const startUsage = cpuUsage();

      const lp = await pp.getLiquidationPrice(group, mangoAccount);
      if (lp == null || lp.lt(ZERO_I80F48())) {
        continue;
      }
      const lpUi = group
        .getPerpMarketByMarketIndex(pp.marketIndex)
        .priceNativeToUi(lp.toNumber());

      const gClone: Group = copy(group);
      gClone.getPerpMarketByMarketIndex(pm.perpMarketIndex)._price = lp;

      const simHealth = toUiDecimalsForQuote(
        mangoAccount.getHealth(gClone, HealthType.maint),
      );

      const now = Date.now();
      const endUsage = cpuUsage(startUsage);

      console.log(
        ` - ${pm.name}, health: ${health.toLocaleString()}, side: ${
          pp.getBasePosition(pm).isPos() ? 'LONG' : 'SHORT'
        }, notional: ${pp
          .getNotionalValueUi(pm)
          .toLocaleString()}, liq price: ${lpUi.toLocaleString()}, sim health: ${simHealth.toLocaleString()}, time ${
          now - then
        }ms, cpu usage ${(endUsage['user'] / 1000).toLocaleString()}ms`,
      );
    }
  }
}

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const wallet = new Wallet(new Keypair());
  const provider = new AnchorProvider(connection, wallet, options);
  const client = MangoClient.connect(provider, CLUSTER, MANGO_V4_ID[CLUSTER], {
    idsSource: 'api',
  });

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const mangoAccounts = await client.getAllMangoAccounts(group, true);
  mangoAccounts.sort((a, b) => b.getEquity(group).cmp(a.getEquity(group)));

  for (const mangoAccount of mangoAccounts) {
    if (mangoAccount.publicKey.equals(new PublicKey(MANGO_ACCOUNT_PK!))) {
      console.log(
        `account https://app.mango.markets/?address=${mangoAccount.publicKey}`,
      );
      await debugUser(client, group, mangoAccount);
    }
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
