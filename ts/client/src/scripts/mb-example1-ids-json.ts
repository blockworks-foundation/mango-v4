import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../client';
import { toUiDecimals } from '../utils';

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.CLUSTER_URL, options);

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

  const group = await client.getGroupForAdmin(admin.publicKey, 0);
  console.log(`${group.toString()}`);

  // create + fetch account
  console.log(`Creating mangoaccount...`);
  const mangoAccount = await client.getOrCreateMangoAccount(
    group,
    user.publicKey,
    0,
    'my_mango_account',
  );
  console.log(`...created/found mangoAccount ${mangoAccount.publicKey}`);
  console.log(mangoAccount.toString(group));

  if (true) {
    await mangoAccount.reload(client, group);
    console.log(
      'mangoAccount.getEquity() ' +
        toUiDecimals(mangoAccount.getEquity().toNumber()),
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
  }

  process.exit();
}

main();
