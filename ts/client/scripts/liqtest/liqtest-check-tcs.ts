import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import { I80F48 } from '../../src/numbers/I80F48';
import { expect } from 'chai';
import { HealthType } from '../../src/accounts/mangoAccount';

//
// This script creates liquidation candidates
//

const GROUP_NUM = Number(process.env.GROUP_NUM || 200);
const CLUSTER = process.env.CLUSTER || 'mainnet-beta';

async function main() {
  const options = AnchorProvider.defaultOptions();
  options.commitment = 'processed';
  options.preflightCommitment = 'finalized';
  const connection = new Connection(process.env.CLUSTER_URL!, options);

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const userWallet = new Wallet(admin);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
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
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);

  const accounts = await client.getMangoAccountsForOwner(
    group,
    admin.publicKey,
  );

  const usdcBank = group.banksMapByName.get('USDC')![0];
  const solBank = group.banksMapByName.get('SOL')![0];

  // LIQEE1 executed up the the margin limit
  {
    const account = ensure(
      accounts.find((account) => account.name == 'LIQTEST, LIQEE1'),
    );
    expect(account.tokenConditionalSwapsActive.length).equal(0);
    expect(Math.round(account.getTokenBalance(usdcBank).toNumber())).equal(
      1000 - 4715,
    );
    expect(account.getHealthRatioUi(group, HealthType.init)).lessThan(1);
  }

  // LIQEE2 executed fully
  {
    const account = ensure(
      accounts.find((account) => account.name == 'LIQTEST, LIQEE2'),
    );
    expect(account.tokenConditionalSwapsActive.length).equal(0);
    expect(Math.round(account.getTokenBalance(solBank).toNumber())).equal(991);
  }

  // LIQEE3 was closed due to expiry
  {
    const account = ensure(
      accounts.find((account) => account.name == 'LIQTEST, LIQEE3'),
    );
    expect(account.tokenConditionalSwapsActive.length).equal(0);
    expect(Math.round(account.getTokenBalance(usdcBank).toNumber())).equal(
      1000000,
    );
  }

  process.exit();
}

function ensure<T>(value: T | undefined): T {
  if (value == null) {
    throw new Error('Value was nullish');
  }
  return value;
}

main();
