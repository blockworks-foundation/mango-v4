import { PublicKey } from '@solana/web3.js';
import { Group } from '../src/accounts/group';
import { ZERO_I80F48 } from '../src/numbers/I80F48';

export interface OraclesFromMangoGroupInterface {
  oraclePk: PublicKey;
  name: string;
  fallbackForOracle: PublicKey | undefined;
  tokenIndex: number | undefined; // todo remove
  perpMarketIndex: number | undefined;
  // todo: add tier when program mango-v4 24.3 is released
}

/**
 * scans mango group for all oracles that need updating
 * includes bank oracle, fallback oracle and perp market oracles
 */
export function getOraclesForMangoGroup(
  group: Group,
): OraclesFromMangoGroupInterface[] {
  // oracles for tokens
  const oracles1: OraclesFromMangoGroupInterface[] = Array.from(
    group.banksMapByName.values(),
  )
    .filter(
      (b) =>
        !(
          b[0].nativeDeposits().eq(ZERO_I80F48()) &&
          b[0].nativeBorrows().eq(ZERO_I80F48()) &&
          b[0].reduceOnly == 1
        ),
    )
    .map((b) => {
      return {
        oraclePk: b[0].oracle,
        name: b[0].name,
        fallbackForOracle: undefined,
        tokenIndex: b[0].tokenIndex,
        perpMarketIndex: undefined,
      };
    });

  // oracles for perp markets
  const oracles2: OraclesFromMangoGroupInterface[] = Array.from(
    group.perpMarketsMapByName.values(),
  ).map((pM) => {
    return {
      oraclePk: pM.oracle,
      name: pM.name,
      fallbackForOracle: undefined,
      tokenIndex: undefined,
      perpMarketIndex: pM.perpMarketIndex,
    };
  });

  // fallback oracles for tokens
  const oracles3: OraclesFromMangoGroupInterface[] = Array.from(
    group.banksMapByName.values(),
  )
    .filter(
      (b) =>
        !(
          b[0].nativeDeposits().eq(ZERO_I80F48()) &&
          b[0].nativeBorrows().eq(ZERO_I80F48()) &&
          b[0].reduceOnly == 1
        ),
    )
    .map((b) => {
      return {
        oraclePk: b[0].fallbackOracle,
        name: b[0].name,
        fallbackForOracle: b[0].oracle,
        tokenIndex: b[0].tokenIndex,
        perpMarketIndex: undefined,
      };
    })
    .filter((item) => !item.oraclePk.equals(PublicKey.default));
  const oracles = oracles1.concat(oracles2).concat(oracles3);
  return oracles;
}
