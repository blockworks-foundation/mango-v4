import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { HealthType } from '../accounts/mangoAccount';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { toUiDecimalsForQuote } from '../utils';

const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'], // use devnet usdc
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
  ['SOL', 'So11111111111111111111111111111111111111112'],
  ['ORCA', 'orcarKHSqC5CDDsGbho8GKvwExejWHxTqGzXgcewB9L'],
  ['MNGO', 'Bb9bsTQa1bGEtQ5KagGkvSHyuLqDWumFUcRqFusFNJWC'],
]);

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.devnet.rpcpool.com',
    options,
  );

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.USER2_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'devnet',
    MANGO_V4_ID['devnet'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  // fetch group
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(group.toString());

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString());

  if (true) {
    await group.reloadAll(client);
    console.log(group.banksMapByName.get('USDC')![0].toString());
    console.log(group.banksMapByName.get('BTC')![0].toString());
  }

  if (false) {
    // deposit and withdraw
    try {
      console.log(`...depositing 0.0005 BTC`);
      await client.tokenDeposit(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS['BTC']),
        0.0005,
      );
      await mangoAccount.reload(client, group);
      console.log(`...withdrawing 5 USDC`);
      await client.tokenWithdraw(
        group,
        mangoAccount,
        new PublicKey(DEVNET_MINTS['USDC']),
        50,
        true,
      );
      await mangoAccount.reload(client, group);
    } catch (error) {
      console.log(error);
    }
  }

  if (true) {
    await mangoAccount.reload(client, group);
    console.log(
      '...mangoAccount.getEquity() ' +
        toUiDecimalsForQuote(mangoAccount.getEquity().toNumber()),
    );
    console.log(
      '...mangoAccount.getCollateralValue() ' +
        toUiDecimalsForQuote(mangoAccount.getCollateralValue().toNumber()),
    );
    console.log(
      '...mangoAccount.accountData["healthCache"].health(HealthType.init) ' +
        toUiDecimalsForQuote(
          mangoAccount.accountData['healthCache']
            .health(HealthType.init)
            .toNumber(),
        ),
    );
    console.log(
      '...mangoAccount.getAssetsVal() ' +
        toUiDecimalsForQuote(mangoAccount.getAssetsVal().toNumber()),
    );
    console.log(
      '...mangoAccount.getLiabsVal() ' +
        toUiDecimalsForQuote(mangoAccount.getLiabsVal().toNumber()),
    );
    console.log(
      '...mangoAccount.getMaxWithdrawWithBorrowForToken(group, "SOL") ' +
        toUiDecimalsForQuote(
          (
            await mangoAccount.getMaxWithdrawWithBorrowForToken(
              group,
              new PublicKey(DEVNET_MINTS['SOL']),
            )
          ).toNumber(),
        ),
    );
    console.log(
      "...mangoAccount.getSerum3MarketMarginAvailable(group, 'BTC/USDC') " +
        toUiDecimalsForQuote(
          mangoAccount
            .getSerum3MarketMarginAvailable(group, 'BTC/USDC')
            .toNumber(),
        ),
    );
    console.log(
      "...mangoAccount.getPerpMarketMarginAvailable(group, 'BTC-PERP') " +
        toUiDecimalsForQuote(
          mangoAccount
            .getPerpMarketMarginAvailable(group, 'BTC-PERP')
            .toNumber(),
        ),
    );
  }

  if (true) {
    await group.reloadAll(client);
    console.log(group.banksMapByName.get('USDC')![0].toString());
    console.log(group.banksMapByName.get('BTC')![0].toString());
  }

  process.exit();
}

main();
