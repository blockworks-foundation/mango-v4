import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import {
  Cluster,
  Connection,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import { BN } from 'bn.js';
import fs from 'fs';
import {
  MarketIndex,
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from '../src/accounts/serum3';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { sendTransaction } from '../src/utils/rpc';

// Env vars
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';

export interface OrderbookL2 {
  bids: number[][];
  asks: number[][];
}

async function rebalancer(): Promise<void> {
  // Load client
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  // Load mango account
  let mangoAccount = await client.getMangoAccount(
    new PublicKey(MANGO_ACCOUNT_PK),
    true,
  );
  console.log(
    `MangoAccount ${mangoAccount.publicKey} for user ${user.publicKey} ${
      mangoAccount.isDelegate(client) ? 'via delegate ' + user.publicKey : ''
    }`,
  );
  await mangoAccount.reload(client);

  // Load group
  const group = await client.getGroup(mangoAccount.group);
  await group.reloadAll(client);
  const usdcBank = group.getFirstBankByMint(
    new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
  );

  // Loop indefinitely
  // eslint-disable-next-line no-constant-condition
  while (true) {
    await group.reloadAll(client);
    mangoAccount = await mangoAccount.reload(client);
    // console.log(mangoAccount.toString(group, true));

    for (const tp of mangoAccount
      .tokensActive()
      .filter((tp) => tp.tokenIndex !== usdcBank.tokenIndex)) {
      const baseBank = group.getFirstBankByTokenIndex(tp.tokenIndex);
      const tokenBalance = tp.balanceUi(baseBank);

      const serum3Markets = Array.from(
        group.serum3MarketsMapByMarketIndex.values(),
      )
        // Find correct $TOKEN/$USDC market
        .filter(
          (serum3Market) =>
            serum3Market.baseTokenIndex === tp.tokenIndex &&
            serum3Market.quoteTokenIndex === usdcBank.tokenIndex,
        );
      if (!serum3Markets) {
        continue;
      }
      const serum3Market = serum3Markets[0];
      const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
        serum3Market.serumMarketExternal.toBase58(),
      )!;
      const maxBaseQuantity = serum3MarketExternal.baseSizeNumberToLots(
        Math.abs(tokenBalance),
      );
      // Skip if quantity is too small
      if (maxBaseQuantity.eq(new BN(0))) {
        // console.log(
        //   ` - Not rebalancing ${tokenBalance} $${baseBank.name}, quantity too small`,
        // );
        continue;
      }
      console.log(`- Rebalancing ${tokenBalance} $${baseBank.name}`);

      // if balance is negative we want to bid at a higher price
      // if balance is positive we want to ask at a lower price
      const price =
        baseBank.uiPrice *
        (1 + (tokenBalance > 0 ? -1 : 1) * baseBank.liquidationFee.toNumber());
      try {
        const sig = await sendTransaction(
          client.program.provider as AnchorProvider,
          [
            ...(await client.serum3PlaceOrderIx(
              group,
              mangoAccount,
              serum3Market.serumMarketExternal,
              tokenBalance > 0 ? Serum3Side.ask : Serum3Side.bid,
              price,
              Math.abs(tokenBalance),
              Serum3SelfTradeBehavior.decrementTake,
              Serum3OrderType.immediateOrCancel,
              new Date().valueOf(),
              10,
            )),
            await client.serum3CancelAllOrdersIx(
              group,
              mangoAccount,
              serum3Market.serumMarketExternal,
            ),
            await client.serum3SettleFundsV2Ix(
              group,
              mangoAccount,
              serum3Market.serumMarketExternal,
            ),
          ],
          group.addressLookupTablesList,
          { prioritizationFee: 1 },
        );

        console.log(` -- sig https://explorer.solana.com/tx/${sig}`);
      } catch (e) {
        console.log(e);
      }
    }

    mangoAccount = await mangoAccount.reload(client);
    const ixs: TransactionInstruction[] = [];
    for (const serum3OoMarketIndex of Array.from(
      mangoAccount.serum3OosMapByMarketIndex.keys(),
    )) {
      const serum3ExternalPk = group.serum3MarketsMapByMarketIndex.get(
        serum3OoMarketIndex as MarketIndex,
      )!.serumMarketExternal;
      // 12502 cu per market
      ixs.push(
        await client.serum3CloseOpenOrdersIx(
          group,
          mangoAccount,
          serum3ExternalPk,
        ),
      );
    }
    if (ixs.length) {
      try {
        const sig = await sendTransaction(
          client.program.provider as AnchorProvider,
          ixs,
          group.addressLookupTablesList,
          { prioritizationFee: 1 },
        );
        console.log(
          ` - closed all serum3 oo accounts, sig https://explorer.solana.com/tx/${sig}`,
        );
      } catch (e) {
        console.log(e);
      }
    }

    // console.log(`${new Date().toUTCString()} sleeping for 1s`);
    await new Promise((r) => setTimeout(r, 1000));
  }
}

rebalancer();
