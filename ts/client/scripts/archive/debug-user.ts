import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../../src/accounts/group';
import { HealthCache } from '../../src/accounts/healthCache';
import { HealthType, MangoAccount } from '../../src/accounts/mangoAccount';
import { PerpMarket } from '../../src/accounts/perp';
import { Serum3Market } from '../../src/accounts/serum3';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { toUiDecimalsForQuote } from '../../src/utils';

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const PAYER_KEYPAIR =
  process.env.PAYER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_NUM = Number(process.env.GROUP_NUM || 0);
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK;
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
      toUiDecimalsForQuote(
        mangoAccount.getAssetsValue(group, HealthType.init)!.toNumber(),
      ),
  );
  console.log(
    'mangoAccount.getLiabsValue() ' +
      toUiDecimalsForQuote(
        mangoAccount.getLiabsValue(group, HealthType.init)!.toNumber(),
      ),
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

    const maxSourceWoFees =
      -maxSourceUi *
      (1 + group.banksMapByName.get(src)![0].loanOriginationFeeRate.toNumber());
    const maxTargetWoFees =
      -maxSourceWoFees *
      (group.banksMapByName.get(src)![0].uiPrice /
        group.banksMapByName.get(tgt)![0].uiPrice);

    const sim = mangoAccount.simHealthRatioWithTokenPositionUiChanges(group, [
      {
        mintPk: group.banksMapByName.get(src)![0].mint,
        uiTokenAmount: maxSourceWoFees,
      },
      {
        mintPk: group.banksMapByName.get(tgt)![0].mint,
        uiTokenAmount: maxTargetWoFees,
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
}

async function main(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(PAYER_KEYPAIR!, 'utf-8'))),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = MangoClient.connect(
    adminProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'api',
    },
  );

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  for (const keypair of [USER_KEYPAIR!]) {
    console.log();
    const user = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(keypair, 'utf-8'))),
    );
    const userWallet = new Wallet(user);
    console.log(`User ${userWallet.publicKey.toBase58()}`);

    const mangoAccounts = await client.getAllMangoAccounts(group);

    for (const mangoAccount of mangoAccounts) {
      if (
        !MANGO_ACCOUNT_PK ||
        mangoAccount.publicKey.equals(new PublicKey(MANGO_ACCOUNT_PK))
      ) {
        console.log();
        console.log(`MangoAccount ${mangoAccount.publicKey}`);
        await debugUser(client, group, mangoAccount);
      }
    }
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
