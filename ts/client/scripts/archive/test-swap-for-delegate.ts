import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../../src/client';
import { MANGO_V4_ID } from '../../src/constants';
import {
  fetchJupiterTransaction,
  fetchRoutes,
  prepareMangoRouterInstructions,
} from '../../src/router';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;
const MANGO_ACCOUNT_PK = process.env.MANGO_ACCOUNT_PK;

const usdcMint = new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v');
const wsolMint = new PublicKey('So11111111111111111111111111111111111111112');

async function x(): Promise<void> {
  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(CLUSTER_URL!, options);
  const user = Keypair.fromSecretKey(
    Buffer.from(
      JSON.parse(
        process.env.KEYPAIR || fs.readFileSync(USER_KEYPAIR!, 'utf-8'),
      ),
    ),
  );
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

  const group = await client.getGroup(
    new PublicKey('78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX'),
  );

  const { bestRoute } = await fetchRoutes(
    usdcMint,
    wsolMint,
    '1',
    undefined,
    undefined,
    undefined,
    user.publicKey,
  );
  const [ixs, alts] =
    bestRoute!.routerName === 'Mango'
      ? await prepareMangoRouterInstructions(
          bestRoute!,
          usdcMint,
          wsolMint,
          user.publicKey,
        )
      : await fetchJupiterTransaction(
          client.connection,
          bestRoute!,
          user.publicKey,
          0,
          usdcMint,
          wsolMint,
        );

  try {
    console.log('hi');
    const sig = await client.marginTrade({
      group: group,
      mangoAccount: await client.getMangoAccount(
        new PublicKey(MANGO_ACCOUNT_PK!),
      ),
      inputMintPk: usdcMint,
      amountIn: 1,
      outputMintPk: wsolMint,
      userDefinedInstructions: ixs,
      userDefinedAlts: alts,
      flashLoanType: { swap: {} },
    });
    console.log('hi');
    console.log(
      ` - marginTrade, sig https://explorer.solana.com/tx/${sig}?cluster=${
        CLUSTER == 'devnet' ? 'devnet' : ''
      }`,
    );
  } catch (error) {
    console.log(error);
  }
}

x();
