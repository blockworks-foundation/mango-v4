import { Magic as PythMagic } from '@pythnetwork/client';
import { AccountInfo, Connection, PublicKey } from '@solana/web3.js';
import SwitchboardProgram from '@switchboard-xyz/sbv2-lite';
import Big from 'big.js';
import BN from 'bn.js';
import { I80F48, I80F48Dto } from '../numbers/I80F48';

const SBV1_DEVNET_PID = new PublicKey(
  '7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU',
);
const SBV1_MAINNET_PID = new PublicKey(
  'DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM',
);

const ORCA_MAINNET_PID = new PublicKey(
  'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc',
);
const ORCA_WHIRLPOOL_LEN = 653;
const ORCA_WHIRLPOOL_DISCRIMINATOR = [63, 149, 209, 12, 225, 128, 99, 9];

const RAYDIUM_MAINNET_PID = new PublicKey(
  'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK',
);
const RAYDIUM_POOL_LEN = 1544;
const RAYDIUM_POOL_DISCRIMINATOR = [247, 237, 227, 245, 215, 195, 222, 70];

export const USDC_MINT_MAINNET = new PublicKey(
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
);
export const SOL_MINT_MAINNET = new PublicKey(
  'So11111111111111111111111111111111111111112',
);

let sbv2DevnetProgram;
let sbv2MainnetProgram;

export enum OracleProvider {
  Pyth,
  Switchboard,
  Stub,
}

export class StubOracle {
  public price: I80F48;
  public deviation: I80F48;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      mint: PublicKey;
      price: I80F48Dto;
      lastUpdateTs: BN;
      lastUpdateSlot: BN;
      deviation: I80F48Dto;
    },
  ): StubOracle {
    return new StubOracle(
      publicKey,
      obj.group,
      obj.mint,
      obj.price,
      obj.lastUpdateTs,
      obj.lastUpdateSlot,
      obj.deviation,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public mint: PublicKey,
    price: I80F48Dto,
    public lastUpdateTs: BN,
    public lastUpdateSlot: BN,
    deviation: I80F48Dto,
  ) {
    this.price = I80F48.from(price);
    this.deviation = I80F48.from(deviation);
  }
}

// https://gist.github.com/microwavedcola1/b741a11e6ee273a859f3ef00b35ac1f0
export function parseSwitchboardOracleV1(accountInfo: AccountInfo<Buffer>): {
  price: number;
  lastUpdatedSlot: number;
  uiDeviation: number;
} {
  const price = accountInfo.data.readDoubleLE(1 + 32 + 4 + 4);
  const lastUpdatedSlot = parseInt(
    accountInfo.data.readBigUInt64LE(1 + 32 + 4 + 4 + 8).toString(),
  );
  const minResponse = accountInfo.data.readDoubleLE(1 + 32 + 4 + 4 + 8 + 8 + 8);
  const maxResponse = accountInfo.data.readDoubleLE(
    1 + 32 + 4 + 4 + 8 + 8 + 8 + 8,
  );
  return { price, lastUpdatedSlot, uiDeviation: maxResponse - minResponse };
}

export function switchboardDecimalToBig(sbDecimal: {
  mantissa: BN;
  scale: number;
}): Big {
  const mantissa = new Big(sbDecimal.mantissa.toString());
  const scale = sbDecimal.scale;
  const oldDp = Big.DP;
  Big.DP = 20;
  const result: Big = mantissa.div(new Big(10).pow(scale));
  Big.DP = oldDp;
  return result;
}

export function parseSwitchboardOracleV2(
  program: SwitchboardProgram,
  accountInfo: AccountInfo<Buffer>,
  oracle: PublicKey,
): { price: number; lastUpdatedSlot: number; uiDeviation: number } {
  try {
    //
    const price = program.decodeLatestAggregatorValue(accountInfo)!.toNumber();
    const lastUpdatedSlot = program
      .decodeAggregator(accountInfo)
      .latestConfirmedRound!.roundOpenSlot!.toNumber();
    const stdDeviation = switchboardDecimalToBig(
      program.decodeAggregator(accountInfo).latestConfirmedRound.stdDeviation,
    );

    return { price, lastUpdatedSlot, uiDeviation: stdDeviation.toNumber() };
    //if oracle is badly configured or didn't publish price at least once
    //decodeLatestAggregatorValue can throw (0 switchboard rounds).
  } catch (e) {
    console.log(`Unable to parse Switchboard Oracle V2: ${oracle}`, e);
    return { price: 0, lastUpdatedSlot: 0, uiDeviation: 0 };
  }
}

/**
 *
 * @param accountInfo
 * @returns ui price
 */
export async function parseSwitchboardOracle(
  oracle: PublicKey,
  accountInfo: AccountInfo<Buffer>,
  connection: Connection,
): Promise<{ price: number; lastUpdatedSlot: number; uiDeviation: number }> {
  if (accountInfo.owner.equals(SwitchboardProgram.devnetPid)) {
    if (!sbv2DevnetProgram) {
      sbv2DevnetProgram = await SwitchboardProgram.loadDevnet(connection);
    }
    return parseSwitchboardOracleV2(sbv2DevnetProgram, accountInfo, oracle);
  }

  if (accountInfo.owner.equals(SwitchboardProgram.mainnetPid)) {
    if (!sbv2MainnetProgram) {
      sbv2MainnetProgram = await SwitchboardProgram.loadMainnet(connection);
    }
    return parseSwitchboardOracleV2(sbv2MainnetProgram, accountInfo, oracle);
  }

  if (
    accountInfo.owner.equals(SBV1_DEVNET_PID) ||
    accountInfo.owner.equals(SBV1_MAINNET_PID)
  ) {
    return parseSwitchboardOracleV1(accountInfo);
  }

  throw new Error(`Should not be reached!`);
}

export function isSwitchboardOracle(accountInfo: AccountInfo<Buffer>): boolean {
  if (
    accountInfo.owner.equals(SBV1_DEVNET_PID) ||
    accountInfo.owner.equals(SBV1_MAINNET_PID) ||
    accountInfo.owner.equals(SwitchboardProgram.devnetPid) ||
    accountInfo.owner.equals(SwitchboardProgram.mainnetPid)
  ) {
    return true;
  }
  return false;
}

export function isPythOracle(accountInfo: AccountInfo<Buffer>): boolean {
  return accountInfo.data.readUInt32LE(0) === PythMagic;
}

export function isOrcaOracle(accountInfo: AccountInfo<Buffer>): boolean {
  for (let i = 0; i < 8; i++) {
    if (accountInfo.data.at(i) !== ORCA_WHIRLPOOL_DISCRIMINATOR[i]) {
      return false;
    }
  }

  return (
    accountInfo.owner.equals(ORCA_MAINNET_PID) &&
    accountInfo.data.length == ORCA_WHIRLPOOL_LEN
  );
}

export function isRaydiumOracle(accountInfo: AccountInfo<Buffer>): boolean {
  for (let i = 0; i < 8; i++) {
    if (accountInfo.data.at(i) !== RAYDIUM_POOL_DISCRIMINATOR[i]) {
      return false;
    }
  }

  return (
    accountInfo.owner.equals(RAYDIUM_MAINNET_PID) &&
    accountInfo.data.length == RAYDIUM_POOL_LEN
  );
}

export function isClmmOracle(accountInfo: AccountInfo<Buffer>): boolean {
  return isOrcaOracle(accountInfo) || isRaydiumOracle(accountInfo);
}

export function isOracleStaleOrUnconfident(
  nowSlot: number,
  maxStalenessSlots: number,
  oracleLastUpdatedSlot: number | undefined,
  deviation: I80F48 | undefined,
  confFilter: I80F48,
  price: I80F48,
): boolean {
  if (
    maxStalenessSlots >= 0 &&
    oracleLastUpdatedSlot &&
    nowSlot > oracleLastUpdatedSlot + maxStalenessSlots
  ) {
    return true;
  }

  if (deviation && deviation.gt(confFilter.mul(price))) {
    return true;
  }

  return false;
}

export function deriveFallbackOracleQuoteKey(
  accountInfo: AccountInfo<Buffer>,
): PublicKey {
  if (isOrcaOracle(accountInfo)) {
    const tokenA = new PublicKey(accountInfo.data.subarray(101, 133));
    const tokenB = new PublicKey(accountInfo.data.subarray(181, 213));
    return clmmQuoteKey(tokenA, tokenB);
  } else if (isRaydiumOracle(accountInfo)) {
    const tokenA = new PublicKey(accountInfo.data.subarray(73, 105));
    const tokenB = new PublicKey(accountInfo.data.subarray(105, 137));
    return clmmQuoteKey(tokenA, tokenB);
  } else {
    return PublicKey.default;
  }
}

function clmmQuoteKey(tokenA: PublicKey, tokenB: PublicKey): PublicKey {
  if (
    tokenA.equals(USDC_MINT_MAINNET) ||
    (tokenA.equals(SOL_MINT_MAINNET) && !tokenB.equals(USDC_MINT_MAINNET))
  ) {
    return tokenA; // inverted
  } else {
    return tokenB;
  }
}
// Assumes oracles.length === fallbacks.length
export async function createFallbackOracleMap(
  conn: Connection,
  oracles: PublicKey[],
  fallbacks: PublicKey[],
): Promise<Map<string, [PublicKey, PublicKey]>> {
  const map: Map<string, [PublicKey, PublicKey]> = new Map();
  const accounts = await conn.getMultipleAccountsInfo(fallbacks);
  for (let i = 0; i < oracles.length; i++) {
    if (accounts[i] === null) {
      map.set(oracles[i].toBase58(), [fallbacks[i], PublicKey.default]);
    } else if (!isClmmOracle(accounts[i]!)) {
      map.set(oracles[i].toBase58(), [fallbacks[i], PublicKey.default]);
    } else {
      const quoteKey = deriveFallbackOracleQuoteKey(accounts[i]!);
      map.set(oracles[i].toBase58(), [fallbacks[i], quoteKey]);
    }
  }
  return map;
}
