import * as dotenv from 'dotenv';
dotenv.config();
import { AnchorProvider, Wallet } from '@project-serum/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import { Group } from '../src/accounts/group';
import { MangoAccount } from '../src/accounts/mangoAccount';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import fs from 'fs';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK || '';

const options = AnchorProvider.defaultOptions();
console.log('cluster', CLUSTER_URL);

const connection = new Connection(CLUSTER_URL!, options);
const user = Keypair.fromSecretKey(
  Buffer.from(
    JSON.parse(process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8')),
  ),
);
const userWallet = new Wallet(user);
const userProvider = new AnchorProvider(connection, userWallet, options);

class TestGroupReload {
  private group: Group;
  public mangoAccount: MangoAccount;
  public client: MangoClient;
  public lastOraclePricesUpdateTs;

  constructor() {
    this.lastOraclePricesUpdateTs = 0;
  }

  async setup() {
    this.client = await MangoClient.connect(
      userProvider,
      CLUSTER,
      MANGO_V4_ID[CLUSTER],
      {},
    );

    // Load mango account
    this.mangoAccount = await this.client.getMangoAccount(
      new PublicKey('HvJqTY8xgH2r2BuiPabgyf4bLLEaLewRGtRETvt7B1RC'),
    );
    await this.mangoAccount.reload(this.client);
    // Load group
    this.group = await this.client.getGroup(this.mangoAccount.group);
    await this.group.reloadAll(this.client); // FIXME: need to refresh
  }

  async getPositionPerp() {
    await this.mangoAccount.reload(this.client);
    if (Date.now() - this.lastOraclePricesUpdateTs > 10_000) {
      console.time('reloading group');
      // await this.group.reloadPerpMarketOraclePrices(this.client);
      await this.group.reloadAll(this.client);
      this.lastOraclePricesUpdateTs = Date.now();
      console.timeEnd('reloading group');
      const nav = this.mangoAccount.getEquity(this.group).toNumber() / 1000000; // HERE
      console.log(`NAV=[${nav.toFixed(2)}]`);
    }
  }

  async run() {
    await this.setup();

    while (true) {
      await this.getPositionPerp();
    }
  }
}

const x = new TestGroupReload();

x.run();
