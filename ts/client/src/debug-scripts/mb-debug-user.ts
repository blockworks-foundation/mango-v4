import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { Group } from '../accounts/group';
import { I80F48 } from '../accounts/I80F48';
import { HealthType, MangoAccount } from '../accounts/mangoAccount';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
import { toUiDecimalsForQuote } from '../utils';

async function debugUser(
  client: MangoClient,
  group: Group,
  mangoAccount: MangoAccount,
) {
  console.log(mangoAccount.toString(group));
  await mangoAccount.reload(client, group);

  console.log(
    'buildFixedAccountRetrieverHealthAccounts ' +
      client
        .buildFixedAccountRetrieverHealthAccounts(group, mangoAccount, [
          group.banksMapByName.get('BTC')[0],
          group.banksMapByName.get('USDC')[0],
        ])
        .map((pk) => pk.toBase58())
        .join(', '),
  );
  console.log(
    'mangoAccount.getEquity() ' +
      toUiDecimalsForQuote(mangoAccount.getEquity().toNumber()),
  );
  console.log(
    'mangoAccount.getHealth(HealthType.init) ' +
      toUiDecimalsForQuote(mangoAccount.getHealth(HealthType.init).toNumber()),
  );
  console.log(
    'mangoAccount.getHealthRatio(HealthType.init) ' +
      mangoAccount.getHealthRatio(HealthType.init).toNumber(),
  );
  console.log(
    'mangoAccount.getHealthRatioUi(HealthType.init) ' +
      mangoAccount.getHealthRatioUi(HealthType.init),
  );
  console.log(
    'mangoAccount.getHealthRatio(HealthType.maint) ' +
      mangoAccount.getHealthRatio(HealthType.maint).toNumber(),
  );
  console.log(
    'mangoAccount.getHealthRatioUi(HealthType.maint) ' +
      mangoAccount.getHealthRatioUi(HealthType.maint),
  );
  console.log(
    'mangoAccount.getCollateralValue() ' +
      toUiDecimalsForQuote(mangoAccount.getCollateralValue().toNumber()),
  );
  console.log(
    'mangoAccount.getAssetsValue() ' +
      toUiDecimalsForQuote(
        mangoAccount.getAssetsValue(HealthType.init).toNumber(),
      ),
  );
  console.log(
    'mangoAccount.getLiabsValue() ' +
      toUiDecimalsForQuote(
        mangoAccount.getLiabsValue(HealthType.init).toNumber(),
      ),
  );

  console.log(group.banksMapByName.get('SOL')[0].mint.toBase58());

  async function getMaxWithdrawWithBorrowForTokenUiWrapper(token) {
    console.log(
      `group.getTokenVaultBalanceByMintUi ${token} ${await group.getTokenVaultBalanceByMintUi(
        client,
        group.banksMapByName.get(token)[0].mint,
      )}`,
    );

    console.log(
      `mangoAccount.getMaxWithdrawWithBorrowForTokenUi(group, ${token}) ` +
        mangoAccount.getMaxWithdrawWithBorrowForTokenUi(
          group,
          group.banksMapByName.get(token)[0].mint,
        ),
    );
    try {
      await client.tokenWithdraw(
        group,
        mangoAccount,
        group.banksMapByName.get(token)[0].mint,
        mangoAccount.getMaxWithdrawWithBorrowForTokenUi(
          group,
          group.banksMapByName.get(token)[0].mint,
        ),
        true,
      );
    } catch (error) {
      console.log(error);
    }
  }
  await getMaxWithdrawWithBorrowForTokenUiWrapper('SOL');
  await getMaxWithdrawWithBorrowForTokenUiWrapper('MSOL');
  await getMaxWithdrawWithBorrowForTokenUiWrapper('USDC');
  await getMaxWithdrawWithBorrowForTokenUiWrapper('BTC');

  console.log(
    'mangoAccount.simHealthRatioWithTokenPositionChanges ' +
      (
        await mangoAccount.simHealthRatioWithTokenPositionChanges(group, [
          {
            mintPk: group.banksMapByName.get('USDC')[0].mint,
            tokenAmount:
              -95_000 *
              Math.pow(10, group.banksMapByName.get('USDC')[0]!.mintDecimals!),
          },
          {
            mintPk: group.banksMapByName.get('BTC')[0].mint,
            tokenAmount:
              4 *
              Math.pow(10, group.banksMapByName.get('BTC')[0]!.mintDecimals!),
          },
        ])
      ).toNumber(),
  );

  function getMaxSourceForTokenSwapWrapper(src, tgt) {
    console.log(
      `getMaxSourceForTokenSwap ${src.padEnd(4)} ${tgt.padEnd(4)} ` +
        mangoAccount
          .getMaxSourceForTokenSwap(
            group,
            group.banksMapByName.get(src)[0].mint,
            group.banksMapByName.get(tgt)[0].mint,
            0.9,
          )
          .div(
            I80F48.fromNumber(
              Math.pow(10, group.banksMapByName.get(src)[0].mintDecimals),
            ),
          )
          .toNumber(),
    );
  }
  getMaxSourceForTokenSwapWrapper('SOL', 'BTC');
  getMaxSourceForTokenSwapWrapper('USDC', 'USDC');
}

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );

  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`${group.toString()}`);

  for (const keypair of [
    process.env.MB_PAYER_KEYPAIR,
    process.env.MB_USER2_KEYPAIR,
  ]) {
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
    for (const mangoAccount of mangoAccounts) {
      if (
        '9B8uwqH8FJqLn9kvGPVb5GEksLvmyXb3B8UKCFtRs5cq' ===
        mangoAccount.publicKey.toBase58()
      ) {
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
