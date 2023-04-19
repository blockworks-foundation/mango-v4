import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fetch from 'node-fetch';
import { MangoClient } from '../src/client';
import { MANGO_V4_ID } from '../src/constants';
import { toNative, toUiDecimalsForQuote } from '../src/utils';

const { MB_CLUSTER_URL } = process.env;

const GROUP_PK = '78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX';

async function buildClient(): Promise<MangoClient> {
  const clientKeypair = new Keypair();

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const clientWallet = new Wallet(clientKeypair);
  const clientProvider = new AnchorProvider(connection, clientWallet, options);

  return await MangoClient.connect(
    clientProvider,
    'mainnet-beta',
    MANGO_V4_ID['mainnet-beta'],
    {
      idsSource: 'get-program-accounts',
    },
  );
}

async function computePriceImpact(
  amount: string,
  inputMint: string,
  outputMint: string,
): Promise<{ outAmount: number; priceImpactPct: number }> {
  const url = `https://quote-api.jup.ag/v4/quote?inputMint=${inputMint}&outputMint=${outputMint}&amount=${amount}&swapMode=ExactIn&slippageBps=10000&onlyDirectRoutes=false&asLegacyTransaction=false`;
  const response = await fetch(url);

  let res = await response.json();
  res = res.data[0];

  return {
    outAmount: parseFloat(res.outAmount),
    priceImpactPct: parseFloat(res.priceImpactPct),
  };
}

async function main() {
  const client = await buildClient();
  const group = await client.getGroup(new PublicKey(GROUP_PK));
  await group.reloadAll(client);

  console.log(
    `${'COIN'.padStart(20)}, ${'Scale'.padStart(8)}, ${'Liq Fee'.padStart(
      6,
    )}, ${'$->coin'.padStart(6)}, ${'coin-$'.padStart(6)}`,
  );

  for (const bank of Array.from(group.banksMapByMint.values())) {
    if (bank[0].name === 'USDC' || bank[0].reduceOnly === true) {
      continue;
    }
    const usdcMint = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

    const pi1 = await computePriceImpact(
      bank[0].depositWeightScaleStartQuote.toString(),
      usdcMint,
      bank[0].mint.toBase58(),
    );
    const inAmount = toNative(
      Math.min(
        Math.floor(
          toUiDecimalsForQuote(bank[0].depositWeightScaleStartQuote) /
            bank[0].uiPrice,
        ),
        99999999999,
      ),
      bank[0].mintDecimals,
    );
    const pi2 = await computePriceImpact(
      inAmount.toString(),
      bank[0].mint.toBase58(),
      usdcMint,
    );
    console.log(
      `${bank[0].name.padStart(20)}, ${(
        '$' +
        toUiDecimalsForQuote(bank[0].depositWeightScaleStartQuote).toString()
      ).padStart(8)}, ${(bank[0].liquidationFee.toNumber() * 100)
        .toFixed(3)
        .padStart(6)}%, ${(pi1.priceImpactPct * 100).toFixed(2)}%, ${(
        pi2.priceImpactPct * 100
      ).toFixed(2)}%`,
    );
  }
}

main();
