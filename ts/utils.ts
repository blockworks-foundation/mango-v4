import { AccountMeta } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';

export function debugAccountMetas(ams: AccountMeta[]) {
  for (const am of ams) {
    console.log(
      `${am.pubkey.toBase58()}, isSigner: ${am.isSigner
        .toString()
        .padStart(5, ' ')}, isWritable - ${am.isWritable
        .toString()
        .padStart(5, ' ')}`,
    );
  }
}
