import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { HealthType, MangoAccount } from '../../src/accounts/mangoAccount';
import {
  MANGO_V4_ID,
  MangoClient,
  toUiDecimalsForQuote,
} from '../../src/index';

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );
  console.log(`User ${userWallet.publicKey.toBase58()}`);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );
  console.log(`${group.toString()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = (await client.getMangoAccountForOwner(
    group,
    user.publicKey,
    0,
  )) as MangoAccount;
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString(group));

  // eslint-disable-next-line no-constant-condition
  if (true) {
    console.log(`...depositing 0.0001 USDC`);
    await client.tokenDeposit(
      group,
      mangoAccount,
      group.banksMapByName.get('USDC')![0].mint,
      10,
    );
    await mangoAccount.reload(client);

    console.log(`...depositing 0.001 SOL`);
    await client.tokenDeposit(
      group,
      mangoAccount,
      group.banksMapByName.get('SOL')![0].mint,
      1,
    );
    await mangoAccount.reload(client);
  }

  await mangoAccount.reload(client);
  console.log(
    'mangoAccount.getEquity() ' +
      toUiDecimalsForQuote(mangoAccount.getEquity(group).toNumber()),
  );
  console.log(
    'mangoAccount.getHealth(HealthType.init) ' +
      toUiDecimalsForQuote(
        mangoAccount.getHealth(group, HealthType.init).toNumber(),
      ),
  );
  console.log(
    'mangoAccount.getHealthRatio(HealthType.init) ' +
      mangoAccount.getHealthRatio(group, HealthType.init).toNumber(),
  );
  console.log(
    'mangoAccount.getCollateralValue() ' +
      toUiDecimalsForQuote(mangoAccount.getCollateralValue(group).toNumber()),
  );
  console.log(
    'mangoAccount.getAssetsVal() ' +
      toUiDecimalsForQuote(mangoAccount.getAssetsValue(group).toNumber()),
  );
  console.log(
    'mangoAccount.getLiabsVal() ' +
      toUiDecimalsForQuote(mangoAccount.getLiabsValue(group).toNumber()),
  );

  console.log(
    "mangoAccount.getMaxWithdrawWithBorrowForToken(group, 'SOL') " +
      toUiDecimalsForQuote(
        (
          await mangoAccount.getMaxWithdrawWithBorrowForToken(
            group,
            group.banksMapByName.get('SOL')![0].mint,
          )
        ).toNumber(),
      ),
  );

  console.log(
    "mangoAccount.getMaxSourceForTokenSwap(group, 'USDC', 'BTC') " +
      (await mangoAccount.getMaxSourceUiForTokenSwap(
        group,
        group.banksMapByName.get('USDC')![0].mint,
        group.banksMapByName.get('BTC')![0].mint,
        0.94,
      )),
  );

  console.log(
    'mangoAccount.simHealthWithTokenPositionChanges ' +
      toUiDecimalsForQuote(
        mangoAccount.simHealthRatioWithTokenPositionUiChanges(group, [
          {
            mintPk: group.banksMapByName.get('USDC')![0].mint,
            uiTokenAmount:
              -20000 *
              Math.pow(10, group.banksMapByName.get('BTC')![0].mintDecimals!),
          },
          {
            mintPk: group.banksMapByName.get('BTC')![0].mint,
            uiTokenAmount:
              1 *
              Math.pow(10, group.banksMapByName.get('BTC')![0].mintDecimals!),
          },
        ]),
      ),
  );

  process.exit();
}

main();
