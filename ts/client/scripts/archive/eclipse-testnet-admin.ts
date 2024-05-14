import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { DefaultTokenRegisterParams } from '../../src/clientIxParamBuilder';
import { MANGO_V4_ID } from '../../src/constants';
import { MangoSignatureStatus } from '../../src/utils/rpc';

const TESTNET_MINTS = new Map([
  ['USDC', 'AkdEhBMvaDD1UbGMD3Hxnr3h5PEL2R8PaCDAssCN28WV'],
]);

// TODO: should these constants be baked right into client.ts or even program?
const NET_BORROWS_LIMIT_NATIVE = 1 * Math.pow(10, 7) * Math.pow(10, 6);

const GROUP_NUM = Number(process.env.GROUP_NUM || 0);

async function main(): Promise<void> {
  let sig: MangoSignatureStatus;

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(
    'https://testnet.dev2.eclipsenetwork.xyz',
    options,
  );

  const admin = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(fs.readFileSync(process.env.ADMIN_KEYPAIR!, 'utf-8')),
    ),
  );
  const adminWallet = new Wallet(admin);
  console.log(`Admin ${adminWallet.publicKey.toBase58()}`);
  const adminProvider = new AnchorProvider(connection, adminWallet, options);
  const client = await MangoClient.connect(
    adminProvider,
    'testnet',
    MANGO_V4_ID['testnet'],
    {
      idsSource: 'get-program-accounts',
    },
  );

  // group
  console.log(`Creating Group...`);
  const insuranceMint = new PublicKey(TESTNET_MINTS.get('USDC')!);
  try {
    await client.groupCreate(GROUP_NUM, true, 0, insuranceMint);
  } catch (error) {
    console.log(error);
  }
  const group = await client.getGroupForCreator(admin.publicKey, GROUP_NUM);
  console.log(`...registered group ${group.publicKey}`);

  // stub usdc oracle + register token 0
  console.log(`Registering USDC...`);
  const usdcDevnetMint = new PublicKey(TESTNET_MINTS.get('USDC')!);

  sig = await client.stubOracleCreate(group, usdcDevnetMint, 1.0);
  const usdcDevnetOracle = (
    await client.getStubOracle(group, usdcDevnetMint)
  )[0];
  console.log(
    `...registered stub oracle ${usdcDevnetOracle}, https://explorer.dev.eclipsenetwork.xyz/tx/${sig.signature}?cluster=testnet`,
  );

  sig = await client.tokenRegister(
    group,
    usdcDevnetMint,
    usdcDevnetOracle.publicKey,
    PublicKey.default,
    0, // tokenIndex
    'USDC',
    {
      ...DefaultTokenRegisterParams,
      initAssetWeight: 1,
      maintAssetWeight: 1,
      initLiabWeight: 1,
      maintLiabWeight: 1,
      liquidationFee: 0,
      netBorrowLimitPerWindowQuote: NET_BORROWS_LIMIT_NATIVE,
    },
  );
  await group.reloadAll(client);
  const bank = group.getFirstBankByMint(usdcDevnetMint);
  console.log(
    `...registered token bank ${bank.publicKey}, https://explorer.dev.eclipsenetwork.xyz/tx/${sig.signature}?cluster=testnet`,
  );
  await group.reloadAll(client);

  process.exit();
}

main();
