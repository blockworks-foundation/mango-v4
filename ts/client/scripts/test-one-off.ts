import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Cluster, Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import {
  fetchJupiterTransaction,
  fetchRoutes,
  prepareMangoRouterInstructions,
} from '../src/router';

const CLUSTER: Cluster =
  (process.env.CLUSTER_OVERRIDE as Cluster) || 'mainnet-beta';
const CLUSTER_URL =
  process.env.CLUSTER_URL_OVERRIDE || process.env.MB_CLUSTER_URL;
const USER_KEYPAIR =
  process.env.USER_KEYPAIR_OVERRIDE || process.env.MB_PAYER_KEYPAIR;

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
    new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
    new PublicKey('So11111111111111111111111111111111111111112'),
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
          new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
          new PublicKey('So11111111111111111111111111111111111111112'),
          user.publicKey,
        )
      : await fetchJupiterTransaction(
          client.connection,
          bestRoute!,
          user.publicKey,
          0,
          new PublicKey('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'),
          new PublicKey('So11111111111111111111111111111111111111112'),
        );

  try {
    console.log('hi');
    const sig = await client.marginTrade({
      group: group,
      mangoAccount: await client.getMangoAccount(
        new PublicKey('BNTDZJQrjNkjFxYAMCdKH2ShSM6Uwc28aAgit7ytVQJc'),
      ),
      inputMintPk: new PublicKey(
        'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      ),
      amountIn: 1,
      outputMintPk: new PublicKey(
        'So11111111111111111111111111111111111111112',
      ),
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
