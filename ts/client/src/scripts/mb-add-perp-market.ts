import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { Serum3Side } from '../accounts/serum3';
import { MangoClient } from '../client';
import { MANGO_V4_ID } from '../constants';

const MAINNET_ORACLES = new Map([
  ['USDT', '3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL'],
  ['BTC', 'GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU'],
  ['ETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['soETH', 'JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB'],
  ['SOL', 'H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG'],
  ['MSOL', 'E4v1BBgoso9s64TQvmyownAVJbhbEPGyzA3qn4n46qj9'],
  ['MNGO', '79wm3jjcPr6RaNQ4DGvP5KxG1mNd3gEBsg6FsNVFezK4'],
]);

const PAYER_KEYPAIR = process.env.MB_PAYER_KEYPAIR || '';

//
// (untested?) script which closes a mango account cleanly, first closes all positions, withdraws all tokens and then closes it
//
async function viewUnownedAccount(userKeypairFile: string) {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://mango.rpcpool.com/0f9acc0d45173b51bf7d7e09c1e5',
    options,
  );

  // user
  // const userWallet = new Wallet(Keypair.generate());
  // const userProvider = new AnchorProvider(connection, userWallet, options);
  // console.log(`User ${userWallet.publicKey.toBase58()}`);

  // admin
  const admin = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(PAYER_KEYPAIR, 'utf-8'))),
  );
  const adminWallet = new Wallet(admin);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  console.log(`Admin ${admin.publicKey.toBase58()}`);

  const client = await MangoClient.connect(
    adminProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {},
    'get-program-accounts',
  );

  // fetch group
  const group = await client.getGroupForCreator(admin.publicKey, 2);
  console.log(`Found group ${group.publicKey.toBase58()}`);

  const btcMainnetOracle = new PublicKey(MAINNET_ORACLES.get('BTC')!);
  console.log(`Registering perp market...`);
  try {
    await client.perpCreateMarket(
      group, // group
      btcMainnetOracle, // oracle
      0, // perpMarketIndex
      'BTC-PERP', // name
      0.1, // oracleConfFilter
      6, // baseDecimals
      1, // quoteLotSize
      10, // baseLotSize
      0.975, // maintAssetWeight
      0.95, // initAssetWeight
      1.025, // maintLiabWeight
      1.05, // initLiabWeight
      0.012, // liquidationFee
      0.0002, // makerFee
      0.0, // takerFee
      0, // feePenalty
      0.05, // minFunding
      0.05, // maxFunding
      100, // impactQuantity
      false, // groupInsuranceFund
      true, // trustedMarket
      0, // settleFeeFlat
      0, // settleFeeAmountThreshold
      0, // settleFeeFractionLowHealth
      0, // settleTokenIndex
    );
    console.log('done');
  } catch (error) {
    console.log(error);
  }

  process.exit();
}

async function main() {
  await viewUnownedAccount(process.env.MB_USER2_KEYPAIR || '');
}

main();
