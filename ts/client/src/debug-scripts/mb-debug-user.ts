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
  }
  await getMaxWithdrawWithBorrowForTokenUiWrapper('SOL');
  await getMaxWithdrawWithBorrowForTokenUiWrapper('MSOL');
  await getMaxWithdrawWithBorrowForTokenUiWrapper('USDC');
  await getMaxWithdrawWithBorrowForTokenUiWrapper('BTC');

  function simHealthRatioWithTokenPositionChangesWrapper(debug, change) {
    console.log(
      `mangoAccount.simHealthRatioWithTokenPositionChanges ${debug}` +
        mangoAccount
          .simHealthRatioWithTokenPositionUiChanges(group, [change])
          .toNumber(),
    );
  }
  simHealthRatioWithTokenPositionChangesWrapper('sol 1  ', {
    mintPk: group.banksMapByName.get('SOL')[0].mint,
    uiTokenAmount: 1,
  });
  simHealthRatioWithTokenPositionChangesWrapper('sol -1  ', {
    mintPk: group.banksMapByName.get('SOL')[0].mint,
    uiTokenAmount: -1,
  });
  simHealthRatioWithTokenPositionChangesWrapper('msol 1  ', {
    mintPk: group.banksMapByName.get('MSOL')[0].mint,
    uiTokenAmount: 1,
  });
  simHealthRatioWithTokenPositionChangesWrapper('msol -1  ', {
    mintPk: group.banksMapByName.get('MSOL')[0].mint,
    uiTokenAmount: -1,
  });
  simHealthRatioWithTokenPositionChangesWrapper('usdc 10  ', {
    mintPk: group.banksMapByName.get('USDC')[0].mint,
    uiTokenAmount: 10,
  });
  simHealthRatioWithTokenPositionChangesWrapper('usdc -10  ', {
    mintPk: group.banksMapByName.get('USDC')[0].mint,
    uiTokenAmount: -10,
  });
  simHealthRatioWithTokenPositionChangesWrapper('btc 0.001  ', {
    mintPk: group.banksMapByName.get('BTC')[0].mint,
    uiTokenAmount: 0.001,
  });
  simHealthRatioWithTokenPositionChangesWrapper('btc -0.001  ', {
    mintPk: group.banksMapByName.get('BTC')[0].mint,
    uiTokenAmount: -0.001,
  });
  simHealthRatioWithTokenPositionChangesWrapper('soETH -0.001  ', {
    mintPk: group.banksMapByName.get('soETH')[0].mint,
    uiTokenAmount: -0.001,
  });

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
