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

export async function findOrCreate<T>(
  entityName: string,
  findMethod: Function,
  findArgs: any[],
  createMethod: Function,
  createArgs: any[],
): Promise<T> {
  let many: T[] = await findMethod(...findArgs);
  let one: T;
  if (many.length > 0) {
    one = many[0];
    return one;
  }
  await createMethod(...createArgs);
  many = await findMethod(...findArgs);
  one = many[0];
  return one;
}
