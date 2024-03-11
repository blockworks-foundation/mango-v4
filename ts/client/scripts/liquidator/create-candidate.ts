import { AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { TokenIndex } from '../../src/accounts/bank';
import {
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../../src/accounts/serum3';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { I80F48 } from '../../src/numbers/I80F48';
import { toUiDecimalsForQuote } from '../../src/utils';

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
      idsSource: 'api',
      prioritizationFee: 100,
      txConfirmationCommitment: 'confirmed',
    },
  );

  // fetch group
  const group = await client.getGroup(GROUP);
  // liquidator's mango account
  const liqee = await client.getMangoAccount(new PublicKey(MANGO_ACCOUNT_PK));
  await client.tokenDeposit(
    group,
    liqee,
    group.getFirstBankByTokenIndex(0 as TokenIndex).mint,
    10,
  );

  let stopTryingSpot = false;
  let withdraw = 1e5;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    await group.reloadAll(client);

    if (!stopTryingSpot) {
      try {
        const serum3Market = group.getSerum3MarketByName('BONK/USDC');
        const external = group.getSerum3ExternalMarket(
          serum3Market.serumMarketExternal,
        );
        await client.serum3PlaceOrder(
          group,
          liqee,
          serum3Market.serumMarketExternal,
          Serum3Side.bid,
          group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex).uiPrice *
            1.1,
          external.baseSizeLotsToNumber(new BN(1)),
          Serum3SelfTradeBehavior.decrementTake,
          Serum3OrderType.immediateOrCancel,
          new Date().valueOf(),
          10,
        );
      } catch (error) {
        stopTryingSpot = true;
      }
    }

    try {
      await client.tokenWithdrawNative(
        group,
        liqee,
        group.getFirstBankByTokenIndex(0 as TokenIndex).mint,
        new BN(withdraw),
        true,
      );
    } catch (error) {
      if (withdraw >= 1) {
        withdraw = withdraw / 2;
      } else {
        throw error;
      }
    }

    await liqee.reload(client);

    // console.log(`...Equity - ${toUiDecimalsForQuote(liqee.getEquity(group))}`);
    console.log(
      `...Maint health - ${toUiDecimalsForQuote(
        liqee.getHealth(group, 'Maint'),
      )}`,
    );
    // console.log(``);

    await new Promise((r) => setTimeout(r, 100));
  }
};

main();
