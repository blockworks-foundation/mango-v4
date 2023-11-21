import { AnchorProvider } from '@coral-xyz/anchor';
import { ReferralProvider } from '@jup-ag/referral-sdk';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import fs from 'fs';

const { MB_CLUSTER_URL, MB_PAYER_KEYPAIR } = process.env;

async function run(): Promise<void> {
  // https://github.com/TeamRaccoons/referral/blob/main/packages/sdk/src/referral.ts

  const payer = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(MB_PAYER_KEYPAIR!, 'utf-8'))),
  );

  const options = AnchorProvider.defaultOptions();
  const connection = new Connection(MB_CLUSTER_URL!, options);

  const rp = new ReferralProvider(connection);
  const raPub = new PublicKey('EV4qhLE2yPKdUPdQ74EWJUn21xT3eGQxG3DRR1g9NNFc');
  const ra = await rp.getReferralAccount(raPub);
  console.log(ra);
  //   {
  //     partner: PublicKey [PublicKey(8SSLjXBEVk9nesbhi9UMCA32uijbVBUqWoKPPQPTekzt)] {
  //       _bn: <BN: 6e85fb2291af2d24cd232cce7f1a61cfea43c0ff79d13d2a2a87fdb066d37a11>
  //     },
  //     project: PublicKey [PublicKey(45ruCyfdRkWpRNGEqWzjCiXRHkZs8WXCLQ67Pnpye7Hp)] {
  //       _bn: <BN: 2dd1ce668720a26d9f36e9c2256228ced5c3cf6103fadb7822ecb87f602b9a8f>
  //     },
  //     shareBps: 9000,
  //     name: 'test111'
  //   }

  const p = await rp.getProject(
    new PublicKey('45ruCyfdRkWpRNGEqWzjCiXRHkZs8WXCLQ67Pnpye7Hp'),
  );
  console.log(p);
  //   {
  //     base: PublicKey [PublicKey(8oW1Poc2q14NgEbtCQAU8tBF6rXMMjf2FGWU9Tfda9rS)] {
  //       _bn: <BN: 73eb52e2aa75f0975741c21546792c026b5bed3870c3999236afd6f30ee71efb>
  //     },
  //     admin: PublicKey [PublicKey(AfQ1oaudsGjvznX4JNEw671hi57JfWo4CWqhtkdgoVHU)] {
  //       _bn: <BN: 8f8f466158ccdc94921985364e38eb5c1fbd3ab1b98c46a7f33c9499d174370b>
  //     },
  //     name: 'Limit Order',
  //     defaultShareBps: 9000
  //   }

  // Create a referral token account tx for a mint, where fees will be accrued
  const tx = await rp.initializeReferralTokenAccount({
    payerPubKey: payer.publicKey,
    referralAccountPubKey: raPub,
    // https://www.coingecko.com/en/coins/myro
    mint: new PublicKey('HhJpBhRRn4g56VsyLuT8DL5Bv31HkXqsrahTTUCZeZg4'),
  });
  console.log(tx);
}

run();
