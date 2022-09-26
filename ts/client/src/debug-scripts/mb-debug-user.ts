import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Cluster, Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../accounts/group';
import { HealthCache } from '../accounts/healthCache';
import { HealthType, MangoAccount } from '../accounts/mangoAccount';
import { PerpMarket } from '../accounts/perp';
import { Serum3Market } from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { toUiDecimalsForQuote } from '../utils';

const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const PAYER_KEYPAIR =
  process.env.PAYER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const GROUP_NUM = Number(process.env.GROUP_NUM || 2);
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';

async function debugUser(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
) {
  console.log(mangoAccount.toString(group));

  await mangoAccount.reload(client, group);

  /////
  const hc = HealthCache.fromMangoAccount(group, mangoAccount);
  console.log(hc.health(HealthType.init));
  console.log(mangoAccount.accountData?.healthCache.health(HealthType.init));
  console.log(`${hc.perpInfos}`);
  console.log(`${mangoAccount.accountData?.healthCache.perpInfos}`);
  return;

  /////

  console.log(
    'buildFixedAccountRetrieverHealthAccounts ' +
      client
        .buildFixedAccountRetrieverHealthAccounts(
          group,
          mangoAccount,
          [
            group.banksMapByName.get('BTC')![0],
            group.banksMapByName.get('USDC')![0],
          ],
          [],
        )
        .map((pk) => pk.toBase58())
        .join(', '),
  );
  console.log(
    'mangoAccount.getEquity() ' +
      toUiDecimalsForQuote(mangoAccount.getEquity()!.toNumber()),
  );
  console.log(
    'mangoAccount.getHealth(HealthType.init) ' +
      toUiDecimalsForQuote(mangoAccount.getHealth(HealthType.init)!.toNumber()),
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
      mangoAccount.getHealthRatio(HealthType.init)!.toNumber(),
  );
  console.log(
    'mangoAccount.getHealthRatioUi(HealthType.init) ' +
      mangoAccount.getHealthRatioUi(HealthType.init),
  );
  console.log(
    'mangoAccount.getHealthRatio(HealthType.maint) ' +
      mangoAccount.getHealthRatio(HealthType.maint)!.toNumber(),
  );
  console.log(
    'mangoAccount.getHealthRatioUi(HealthType.maint) ' +
      mangoAccount.getHealthRatioUi(HealthType.maint),
  );
  console.log(
    'mangoAccount.getCollateralValue() ' +
      toUiDecimalsForQuote(mangoAccount.getCollateralValue()!.toNumber()),
  );
  console.log(
    'mangoAccount.getAssetsValue() ' +
      toUiDecimalsForQuote(
        mangoAccount.getAssetsValue(HealthType.init)!.toNumber(),
      ),
  );
  console.log(
    'mangoAccount.getLiabsValue() ' +
      toUiDecimalsForQuote(
        mangoAccount.getLiabsValue(HealthType.init)!.toNumber(),
      ),
  );

  async function getMaxWithdrawWithBorrowForTokenUiWrapper(token) {
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

  function simHealthRatioWithTokenPositionChangesWrapper(debug, change) {
    console.log(
      `mangoAccount.simHealthRatioWithTokenPositionChanges ${debug}` +
        mangoAccount.simHealthRatioWithTokenPositionUiChanges(group, [change]),
    );
  }
  for (const srcToken of Array.from(group.banksMapByName.keys())) {
    simHealthRatioWithTokenPositionChangesWrapper(`${srcToken} 1  `, {
      mintPk: group.banksMapByName.get(srcToken)![0].mint,
      uiTokenAmount: 1,
    });
    simHealthRatioWithTokenPositionChangesWrapper(`${srcToken} -1  `, {
      mintPk: group.banksMapByName.get(srcToken)![0].mint,
      uiTokenAmount: -1,
    });
  }

  function getMaxSourceForTokenSwapWrapper(src, tgt) {
    console.log(
      `getMaxSourceForTokenSwap ${src.padEnd(4)} ${tgt.padEnd(4)} ` +
        mangoAccount.getMaxSourceUiForTokenSwap(
          group,
          group.banksMapByName.get(src)![0].mint,
          group.banksMapByName.get(tgt)![0].mint,
          1,
        ),
    );
  }
  for (const srcToken of Array.from(group.banksMapByName.keys())) {
    for (const tgtToken of Array.from(group.banksMapByName.keys())) {
      // if (srcToken === 'SOL')
      // if (tgtToken === 'MSOL')
      getMaxSourceForTokenSwapWrapper(srcToken, tgtToken);
    }
  }

  function getMaxForPerpWrapper(perpMarket: PerpMarket) {
    console.log(
      `getMaxQuoteForPerpBidUi ${perpMarket.name} ` +
        mangoAccount.getMaxQuoteForPerpBidUi(
          group,
          perpMarket.name,
          perpMarket.price,
        ),
    );
    console.log(
      `getMaxBaseForPerpAskUi ${perpMarket.name} ` +
        mangoAccount.getMaxBaseForPerpAskUi(
          group,
          perpMarket.name,
          perpMarket.price,
        ),
    );
  }
  for (const perpMarket of Array.from(group.perpMarketsMap.values())) {
    getMaxForPerpWrapper(perpMarket);
  }

  function getMaxForSerum3Wrapper(serum3Market: Serum3Market) {
    // if (serum3Market.name !== 'SOL/USDC') return;
    console.log(
      `getMaxQuoteForSerum3BidUi ${serum3Market.name} ` +
        mangoAccount.getMaxQuoteForSerum3BidUi(
          group,
          serum3Market.serumMarketExternal,
        ),
    );
    console.log(
      `getMaxBaseForSerum3AskUi ${serum3Market.name} ` +
        mangoAccount.getMaxBaseForSerum3AskUi(
          group,
          serum3Market.serumMarketExternal,
        ),
    );
  }
  for (const serum3Market of Array.from(
    group.serum3MarketsMapByExternal.values(),
  )) {
    getMaxForSerum3Wrapper(serum3Market);
  }
}

async function main() {
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
    {},
    'get-program-accounts',
  );

  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  for (const keypair of [USER_KEYPAIR!]) {
    console.log();
    const user = Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(keypair, 'utf-8'))),
    );
    const userWallet = new Wallet(user);
    console.log(`User ${userWallet.publicKey.toBase58()}`);

    const mangoAccounts = await client.getMangoAccountsForOwner(
      group,
      user.publicKey,
    );

    // const mangoAccounts = await Promise.all([
    //   await client.getMangoAccount({
    //     publicKey: new PublicKey(
    //       '6mqHfpJqnXcu6RgDYZSVW9CQXQPFyRYhgvdzvWXN9mPW',
    //     ),
    //   } as any),
    // ]);

    for (const mangoAccount of mangoAccounts) {
      console.log(`MangoAccount ${mangoAccount.publicKey}`);
      // if (mangoAccount.name === 'PnL Test') {
      await debugUser(client, group, mangoAccount);
      // }
    }
  }

  process.exit();
}

try {
  main();
} catch (error) {
  console.log(error);
}
