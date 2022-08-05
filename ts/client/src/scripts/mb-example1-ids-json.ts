import { AnchorProvider, Wallet } from '@frahman5/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { HealthType } from '../accounts/mangoAccount';
import { AccountSize, MangoClient } from '../index';
import { toUiDecimals } from '../utils';

//
// example script shows usage of ids json (saves havint to do gpa)
//

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connectForGroupName(
    userProvider,
    'mainnet-beta.microwavedcola' /* Use ids json instead of getProgramAccounts */,
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        fs.readFileSync(process.env.MANGO_MAINNET_PAYER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const group = await client.getGroupForCreator(admin.publicKey, 0);
  console.log(`${group.toString()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString(group));

  if (false) {
    console.log(`...depositing 10 USDC`);
    await client.tokenDeposit(group, mangoAccount, 'USDC', 10);
    await mangoAccount.reload(client, group);

    console.log(`...depositing 1 SOL`);
    await client.tokenDeposit(group, mangoAccount, 'SOL', 1);
    await mangoAccount.reload(client, group);
  }

  await mangoAccount.reload(client, group);
  console.log(
    'mangoAccount.getEquity() ' +
      toUiDecimals(mangoAccount.getEquity().toNumber()),
  );
  console.log(
    'mangoAccount.getHealth(HealthType.init) ' +
      toUiDecimals(mangoAccount.getHealth(HealthType.init).toNumber()),
  );
  console.log(
    'mangoAccount.getHealthRatio(HealthType.init) ' +
      mangoAccount.getHealthRatio(HealthType.init).toNumber(),
  );
  console.log(
    'mangoAccount.getCollateralValue() ' +
      toUiDecimals(mangoAccount.getCollateralValue().toNumber()),
  );
  console.log(
    'mangoAccount.getAssetsVal() ' +
      toUiDecimals(mangoAccount.getAssetsVal().toNumber()),
  );
  console.log(
    'mangoAccount.getLiabsVal() ' +
      toUiDecimals(mangoAccount.getLiabsVal().toNumber()),
  );

  console.log(
    "mangoAccount.getMaxWithdrawWithBorrowForToken(group, 'SOL') " +
      toUiDecimals(
        (
          await mangoAccount.getMaxWithdrawWithBorrowForToken(group, 'SOL')
        ).toNumber(),
      ),
  );
  // let withdraw = mangoAccount
  //   .getNative(group.banksMap.get('SOL')!)
  //   .add(mangoAccount.getMaxWithdrawWithBorrowForToken(group, 'SOL'))
  //   .toNumber();
  // console.log(` Withdrawing ${toUiDecimals(withdraw)} SOL`);
  // await client.tokenWithdraw2(group, mangoAccount, 'SOL', withdraw, true);
  // console.log(` Depositing ${toUiDecimals(withdraw)} SOL`);
  // await client.tokenDeposit(group, mangoAccount, 'SOL', withdraw);

  console.log(
    "mangoAccount.getMaxSourceForTokenSwap(group, 'USDC', 'BTC') " +
      toUiDecimals(
        (
          await mangoAccount.getMaxSourceForTokenSwap(
            group,
            'USDC',
            'BTC',
            0.94,
          )
        ).toNumber(),
      ),
  );

  console.log(
    'mangoAccount.simHealthWithTokenPositionChanges ' +
      toUiDecimals(
        (
          await mangoAccount.simHealthWithTokenPositionChanges(group, [
            {
              tokenName: 'USDC',
              tokenAmount:
                -20_000 *
                Math.pow(10, group.banksMap.get('BTC')?.mintDecimals!),
            },
            {
              tokenName: 'BTC',
              tokenAmount:
                1 * Math.pow(10, group.banksMap.get('BTC')?.mintDecimals!),
            },
          ])
        ).toNumber(),
      ),
  );

  process.exit();
}

main();
