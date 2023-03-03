import { AnchorProvider } from '@coral-xyz/anchor';
import { getMint } from '@solana/spl-token';
import { Connection, PublicKey } from '@solana/web3.js';
import fetch from 'node-fetch';

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, MB_PAYER3_KEYPAIR, MINT } =
  process.env;

async function main() {
  const mintPk = new PublicKey(
    MINT ??
      // 'RLBxxFkseAZ4RgJH3Sqn8jXxhmGoz9jWxDNJMh8pL7a', // rlb
      //  'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac', // mngo
      'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // usdc
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);
  const mint = await getMint(connection, mintPk);

  // does the mint have an authority?
  console.log(`Mint has freezeAuthority - ${mint.freezeAuthority != null}`);
  console.log(`Mint has mintAuthority - ${mint.mintAuthority != null}`);

  const supplyUi = Number(mint.supply) / Math.pow(10, mint.decimals);

  // how does the token holder distribution look like?
  const res = await connection.getTokenLargestAccounts(mintPk);
  console.log(`Top 20 token distribution`);
  for (const account of res.value) {
    console.log(
      `- ${account.address.toBase58().padStart(45)} - ${(
        (account.uiAmount! * 100) /
        supplyUi
      ).toFixed(2)}%`,
    );
  }

  // are their amm pools? how is their tvl?
  // maybe birdeye can provide this?

  // give stats on max,min swap that can be performed at various slippage
  // mngo cloud router doesnt work for most mints, maybe use jup api?
  const wsolMint = 'So11111111111111111111111111111111111111112';
  async function swap(
    priceImpact: number,
    inputMint: string,
    outputMint: string,
  ): Promise<void> {
    const response = await fetch(
      `https://api.mngo.cloud/router/v1/depth?inputMint=${inputMint}&outputMint=${outputMint}&priceImpactLimit=${priceImpact}`,
    );
    const res = await response.json();
    console.log(
      `at price impact of ${(priceImpact * 100)
        .toString()
        .padStart(
          3,
        )}%, ${res.maxInput.toLocaleString()} ${inputMint} can be swapped to ${res.minOutput.toLocaleString()} ${outputMint}`,
    );
  }
  for (const priceImpact of [0.01, 0.1, 0.2]) {
    swap(priceImpact, wsolMint, mintPk.toBase58());
    swap(priceImpact, mintPk.toBase58(), wsolMint);
  }

  // what was the volume traded yday
  // get from jup api?

  // how does the oracle look like?

  // how was the price volatility yday?
}

main();
