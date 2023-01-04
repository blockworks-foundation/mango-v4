import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair } from '@solana/web3.js';
import * as dotenv from 'dotenv';
import fs from 'fs';
import { PerpMarket } from '../accounts/perp';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';
dotenv.config();

//
// (untested?) script which closes a mango account cleanly, first closes all positions, withdraws all tokens and then closes it
//
async function editPerpMarket(perpMarketName: string) {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(process.env.MB_CLUSTER_URL!, options);

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.MB_PAYER_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
  );
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const pm: PerpMarket = group.getPerpMarketByName(perpMarketName);

  const signature = await client.perpEditMarket(
    group,
    pm.perpMarketIndex,
    pm.oracle,
    {
      confFilter: pm.oracleConfig.confFilter.toNumber(),
      maxStalenessSlots: null,
    },
    pm.baseDecimals,
    pm.maintAssetWeight.toNumber(),
    pm.initAssetWeight.toNumber(),
    pm.maintLiabWeight.toNumber(),
    pm.initLiabWeight.toNumber(),
    pm.liquidationFee.toNumber(),
    pm.makerFee.toNumber(),
    pm.takerFee.toNumber(),
    pm.feePenalty,
    pm.minFunding.toNumber(),
    pm.maxFunding.toNumber(),
    // pm.impactQuantity.toNumber(),
    1,
    pm.groupInsuranceFund,
    pm.trustedMarket,
    pm.settleFeeFlat,
    pm.settleFeeAmountThreshold,
    pm.settleFeeFractionLowHealth,
    null,
    null,
    null,
    null,
    null,
    null,
  );

  console.log('Tx Successful:', signature);

  process.exit();
}

async function main() {
  await editPerpMarket('BTC-PERP');
}

main();
