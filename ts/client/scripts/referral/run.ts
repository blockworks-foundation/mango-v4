import { Connection, PublicKey } from '@solana/web3.js';

import { useReferral } from './referral';

async function main() {
  const r = await useReferral(new Connection(process.env.MB_CLUSTER_URL!));
  const sig = await r.initializeProject({
    basePubKey: new PublicKey('4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg'),
    adminPubKey: new PublicKey('8SSLjXBEVk9nesbhi9UMCA32uijbVBUqWoKPPQPTekzt'),
    name: 'mango-v4',
    defaultShareBps: 1,
  });
  console.log(sig);
}

main();
