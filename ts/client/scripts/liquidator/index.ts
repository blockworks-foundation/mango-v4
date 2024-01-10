import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../../src/accounts/bank';
import { Group } from '../../src/accounts/group';
import { HealthCache } from '../../src/accounts/healthCache';
import { MangoAccount } from '../../src/accounts/mangoAccount';
import { PerpMarketIndex, PerpOrderSide } from '../../src/accounts/perp';
import { MarketIndex } from '../../src/accounts/serum3';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { I80F48, ONE_I80F48, ZERO_I80F48 } from '../../src/numbers/I80F48';
import { MangoSignatureStatus } from '../../src/utils/rpc';

const GROUP = new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX');
const CLUSTER = process.env.CLUSTER || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';
const LIAB_LIMIT = I80F48.fromNumber(
  Math.min(parseFloat(process.env.LIAB_LIMIT || '0.9'), 1),
);

const main = async (): Promise<void> => {
  const options = AnchorProvider.defaultOptions();
  options.commitment = 'processed';
  options.preflightCommitment = 'finalized';
  const connection = new Connection(CLUSTER_URL!, options);

  const keypair = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(USER_KEYPAIR!, 'utf-8'))),
  );
  const userWallet = new Wallet(keypair);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = MangoClient.connect(
    userProvider,
    CLUSTER as Cluster,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
      prioritizationFee: 100,
      txConfirmationCommitment: 'confirmed',
    },
  );

  // fetch group
  const group = await client.getGroup(GROUP);
  // liquidator's mango account
  const liquidatorMangoAccount = await client.getMangoAccount(
    new PublicKey(MANGO_ACCOUNT_PK),
  );
  if (!liquidatorMangoAccount) {
    throw new Error('liquidatorMangoAccount not found');
  }
  const mangoAccounts = await client.getAllMangoAccounts(group, true);

  // loop over all mangoAccounts and find liquidable ones
  for (const mangoAccount of mangoAccounts) {
    if (!isLiquidable(mangoAccount, group)) {
      continue;
    }

    console.log(`Attempting to liquidate ${mangoAccount.publicKey.toBase58()}`);

    try {
      liquidateAccount(mangoAccount, liquidatorMangoAccount, group, client);
    } catch (e) {
      console.error(
        `Error liquidating ${mangoAccount.publicKey.toBase58()}, ${e}`,
      );
    }
  }
};

main();

function isLiquidable(mangoAccount: MangoAccount, group: Group): boolean {
  return (
    mangoAccount.getHealth(group, 'Init').isNeg() ||
    mangoAccount.getHealth(group, 'Maint').isNeg()
  );
}

async function liquidateAccount(
  mangoAccount: MangoAccount,
  liquidatorMangoAccount: MangoAccount,
  group: Group,
  client: MangoClient,
): Promise<void> {
  // Phase 1?
  try {
    // cancel all perp open orders
    await Promise.all(
      mangoAccount.perpOpenOrders.map((perpOo) => {
        return client.perpForceClosePosition(
          group,
          perpOo.orderMarket as PerpMarketIndex,
          mangoAccount,
          liquidatorMangoAccount,
        );
      }),
    );
  } catch (e) {
    console.error(`Error cancelling perp orders: ${e}`);
  }

  await mangoAccount.reload(client);
  if (!isLiquidable(mangoAccount, group)) {
    throw new Error('Account is no longer liquidable');
  }

  try {
    // cancel all serum open orders
    await Promise.all(
      Object.entries(mangoAccount.serum3OosMapByMarketIndex).map(([mktIdx]) => {
        return client.serum3LiqForceCancelOrders(
          group,
          mangoAccount,
          group.getSerum3MarketByMarketIndex(parseFloat(mktIdx) as MarketIndex)
            .publicKey,
        );
      }),
    );
  } catch (e) {
    console.error(`Error cancelling sersum open orders: ${e}`);
  }

  await mangoAccount.reload(client);
  if (!isLiquidable(mangoAccount, group)) {
    throw new Error('Account is no longer liquidable');
  }

  const liqorHealthCache = HealthCache.fromMangoAccount(
    group,
    liquidatorMangoAccount,
  );

  const liqeeHealthCache = HealthCache.fromMangoAccount(
    group,
    liquidatorMangoAccount,
  );

  // Phase 2?
  // TODO: should we return if this succeeds?
  await liquidatePerpsBaseOrPosPnl(
    mangoAccount,
    liquidatorMangoAccount,
    liqorHealthCache,
    liqeeHealthCache,
    group,
    client,
  );

  // TODO: should we return if this succeeds?
  await liquidateToken(
    mangoAccount,
    liquidatorMangoAccount,
    liqorHealthCache,
    liqeeHealthCache,
    group,
    client,
  );

  // Phase 3?
  // TODO: should we return if this succeeds?
  await liquidatePerpsNegPnl(
    mangoAccount,
    liquidatorMangoAccount,
    group,
    client,
  );

  // TODO: should we return if this succeeds?
  await liquidateTokenBankruptcy(
    mangoAccount,
    liquidatorMangoAccount,
    liqorHealthCache,
    group,
    client,
  );
}

async function liquidateTokenBankruptcy(
  liqeeMangoAccount: MangoAccount,
  liqorMangoAccount: MangoAccount,
  liqorHealthCache: HealthCache,
  group: Group,
  client: MangoClient,
): Promise<void> {
  const tokens = liqeeMangoAccount
    .tokensActive()
    .map((t) => {
      const bank = group.getFirstBankByTokenIndex(t.tokenIndex);
      const price = bank._price;
      if (!price) {
        throw new Error('price not found in liquidateTokenBankruptcy');
      }
      const liabUsdcEquivalent = t.balance(bank).mul(price);

      return {
        ...t,
        price,
        liabUsdcEquivalent,
      };
    })
    .sort((a, b) => b.liabUsdcEquivalent.sub(a.liabUsdcEquivalent).toNumber());

  const tokenLiab = tokens[0];

  const assetBank = group.getFirstBankByTokenIndex(0 as TokenIndex); // USDC
  const liabBank = group.getFirstBankByTokenIndex(
    tokenLiab.tokenIndex as TokenIndex,
  );
  if (!assetBank?._price || !liabBank?._price) {
    throw new Error('asset price or liab bank price not found');
  }
  // TODO: check if this is correct
  const price = assetBank._price.div(liabBank._price);

  const maxLiabTransfer = liqorHealthCache.getMaxSwapSourceForHealthRatio(
    liabBank,
    assetBank,
    price,
    LIAB_LIMIT, // TODO: is this correct? what is a good default for this?
  );

  await client.tokenLiqBankruptcy(
    group,
    liqeeMangoAccount,
    liqorMangoAccount,
    assetBank.mint,
    liabBank.mint,
    maxLiabTransfer,
  );
}

async function liquidateToken(
  liqeeMangoAccount: MangoAccount,
  liqorMangoAccount: MangoAccount,
  liqorHealthCache: HealthCache,
  liqeeHealthCache: HealthCache,
  group: Group,
  client: MangoClient,
): Promise<void> {
  let minNet = ZERO_I80F48();
  let minNetIndex = -1;
  let maxNet = ZERO_I80F48();
  let maxNetIndex = -1;

  for (const [i, token] of liqeeMangoAccount.tokensActive().entries()) {
    const bank = group.getFirstBankByTokenIndex(token.tokenIndex);
    const price = bank._price;
    if (!price) {
      throw new Error('price not found');
    }
    const netDeposit = token.deposits(bank).sub(token.borrows(bank)).mul(price);

    if (netDeposit.lt(minNet)) {
      minNet = netDeposit;
      minNetIndex = i;
    } else if (netDeposit.gt(maxNet)) {
      maxNet = netDeposit;
      maxNetIndex = i;
    }
  }

  if (minNetIndex == -1) {
    throw new Error('min net index neg 1');
  }

  const assetBank = group.getFirstBankByTokenIndex(maxNetIndex as TokenIndex);
  const liabBank = group.getFirstBankByTokenIndex(minNetIndex as TokenIndex);
  if (!assetBank?._price || !liabBank?._price) {
    throw new Error('asset price or liab bank price not found');
  }
  // TODO: check if this is correct
  const price = assetBank._price.div(liabBank._price);

  const maxLiabTransfer = liqorHealthCache.getMaxSwapSourceForHealthRatio(
    liabBank,
    assetBank,
    price,
    LIAB_LIMIT, // TODO: is this correct? what is a good default for this?
  );

  await client.liqTokenWithToken(
    group,
    liqorMangoAccount,
    liqeeMangoAccount,
    assetBank.publicKey,
    liabBank.publicKey,
    maxLiabTransfer.toNumber(),
  );
}

async function liquidatePerpsBaseOrPosPnl(
  liqeeMangoAccount: MangoAccount,
  liqorMangoAccount: MangoAccount,
  liqorHealthCache: HealthCache,
  liqeeHealthCache: HealthCache,
  group: Group,
  client: MangoClient,
): Promise<MangoSignatureStatus> {
  const sortedPerpPositions = liqeeMangoAccount
    .perpActive()
    .map((pp) => {
      const perpMarket = group.getPerpMarketByMarketIndex(pp.marketIndex);
      const basePos = pp.getBasePositionUi(perpMarket);
      const quotePos = pp.getQuotePositionUi(perpMarket);
      const baseVal = basePos * perpMarket._uiPrice;

      return {
        ...pp,
        marketIndex: pp.marketIndex,
        perpMarket,
        basePos,
        quotePos,
        baseVal,
        quoteVal: quotePos,
        side: pp.getBasePosition(perpMarket).isNeg()
          ? PerpOrderSide.ask
          : PerpOrderSide.bid,
      };
    })
    .sort((a, b) => {
      return b.baseVal - a.baseVal;
    });
  const highestValuePerpPosition = sortedPerpPositions[0];

  const maxBaseTransfer = liqorHealthCache.getMaxPerpForHealthRatio(
    highestValuePerpPosition.perpMarket,
    highestValuePerpPosition.perpMarket._price,
    highestValuePerpPosition.side,
    LIAB_LIMIT, // TODO: is this correct? what is a good default for this?
  );

  const maxPerpUnsettledLeverage = I80F48.fromNumber(0.95);
  const perpUnsettledCost = ONE_I80F48()
    .sub(highestValuePerpPosition.perpMarket.initOverallAssetWeight)
    .min(maxPerpUnsettledLeverage);

  const maxUsdcBorrow = liqorMangoAccount.getMaxWithdrawWithBorrowForToken(
    group,
    group.getFirstBankForPerpSettlement().mint,
  );
  const allowedUsdcBorrow = I80F48.fromNumber(0.25).mul(maxUsdcBorrow);
  const maxPnlTransfer = allowedUsdcBorrow.div(perpUnsettledCost);

  return await client.perpLiqBaseOrPositivePnl(
    group,
    liqeeMangoAccount,
    liqorMangoAccount,
    highestValuePerpPosition.perpMarket.perpMarketIndex,
    maxBaseTransfer.toNumber(),
    maxPnlTransfer.toNumber(),
  );
}

async function liquidatePerpsNegPnl(
  liqeeMangoAccount: MangoAccount,
  liqorMangoAccount: MangoAccount,
  group: Group,
  client: MangoClient,
): Promise<MangoSignatureStatus> {
  const sortedPerpPositions = liqeeMangoAccount
    .perpActive()
    .map((pp) => {
      const perpMarket = group.getPerpMarketByMarketIndex(pp.marketIndex);
      const quotePos = pp.getQuotePositionUi(perpMarket);
      return {
        ...pp,
        quotePos,
      };
    })
    .filter((pp) => {
      return pp.quotePos >= 0 ? false : true;
    })
    .sort((a, b) => {
      return a.quotePos - b.quotePos;
    });

  const mostNegPerpPosition = sortedPerpPositions[0];

  return await client.perpLiqNegativePnlOrBankruptcy(
    group,
    liqeeMangoAccount,
    liqorMangoAccount,
    mostNegPerpPosition.marketIndex,
  );
}
