import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import { MangoClient } from '../../client';
import { MANGO_V4_ID } from '../../constants';

// For easy switching between mainnet and devnet, default is mainnet
const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';

async function main() {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);

  // Throwaway keypair
  const user = new Keypair();
  const userWallet = new Wallet(user);
  const userProvider = new AnchorProvider(connection, userWallet, options);
  const client = await MangoClient.connect(
    userProvider,
    CLUSTER,
    MANGO_V4_ID[CLUSTER],
    {
      idsSource: 'get-program-accounts',
    },
  );

  // Load mango account
  let mangoAccount = await client.getMangoAccountForPublicKey(
    new PublicKey(MANGO_ACCOUNT_PK),
  );
  await mangoAccount.reload(client);

  // Load group for mango account
  const group = await client.getGroup(mangoAccount.group);
  await group.reloadAll(client);

  // Log OB
  const perpMarket = group.getPerpMarketByName('BTC-PERP');
  while (true) {
    await new Promise((r) => setTimeout(r, 2000));
    console.clear();
    console.log(await perpMarket.logOb(client));
  }
}

main();
