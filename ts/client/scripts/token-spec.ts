import { AnchorProvider } from '@coral-xyz/anchor';
import { getMint } from '@solana/spl-token';
import { Connection, PublicKey } from '@solana/web3.js';

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR, MB_PAYER3_KEYPAIR, MINT } =
  process.env;

async function main() {
  const mintPk = new PublicKey(
    MINT ?? 'RLBxxFkseAZ4RgJH3Sqn8jXxhmGoz9jWxDNJMh8pL7a',
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);
  const mint = await getMint(connection, mintPk);

  // does the mint have an authority?
  console.log(`Mint has freezeAuthority - ${mint.freezeAuthority != null}`);
  console.log(`Mint has mintAuthority - ${mint.mintAuthority != null}`);

  // how does the token holder distribution look like?
  const res = await connection.getTokenLargestAccounts(mintPk);
  console.log(res);

  // are their amm pools? how is their tvl?
  // 11111111111112&outputMint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v&priceImpactLimit=0.01' | jq
  // what was the volume traded yday
  // how does the oracle look like?
  // how was the price volatility yday?
}

main();
