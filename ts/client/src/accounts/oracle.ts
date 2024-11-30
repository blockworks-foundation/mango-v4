import { AnchorProvider, Program } from '@coral-xyz/anchor';
import { parsePriceData, Magic as PythMagic } from '@pythnetwork/client';
import { AccountInfo, Connection, PublicKey } from '@solana/web3.js';
import { ON_DEMAND_MAINNET_PID } from '@switchboard-xyz/on-demand';
import SwitchboardProgram from '@switchboard-xyz/sbv2-lite';
import Big from 'big.js';
import BN from 'bn.js';
import { Program as Anchor30Program } from 'switchboard-anchor';
import { DEFAULT_RECEIVER_PROGRAM_ID } from '../constants';

import { I80F48, I80F48Dto } from '../numbers/I80F48';
import { toUiDecimals } from '../utils';
import {
  IDL,
  PythSolanaReceiver as PythSolanaReceiverProgram,
} from './pyth_solana_receiver';

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
export let sbOnDemandProgram;
let pythSolanaReceiverProgram;

export enum OracleProvider {
  Pyth, // V1
  PythV2, // V2
  Switchboard, // V1+V2
  SwitchboardOnDemand, // On Demand
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
  provider: OracleProvider;
} {
  const price = accountInfo.data.readDoubleLE(1 + 32 + 4 + 4);
  const lastUpdatedSlot = parseInt(
    accountInfo.data.readBigUInt64LE(1 + 32 + 4 + 4 + 8).toString(),
  );
  const minResponse = accountInfo.data.readDoubleLE(1 + 32 + 4 + 4 + 8 + 8 + 8);
  const maxResponse = accountInfo.data.readDoubleLE(
    1 + 32 + 4 + 4 + 8 + 8 + 8 + 8,
  );
  return {
    price,
    lastUpdatedSlot,
    uiDeviation: maxResponse - minResponse,
    provider: OracleProvider.Switchboard,
  };
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
): {
  price: number;
  lastUpdatedSlot: number;
  uiDeviation: number;
  provider: OracleProvider;
} {
  try {
    //
    const price = program.decodeLatestAggregatorValue(accountInfo)!.toNumber();
    const lastUpdatedSlot = program
      .decodeAggregator(accountInfo)
      .latestConfirmedRound!.roundOpenSlot!.toNumber();
    const stdDeviation = switchboardDecimalToBig(
      program.decodeAggregator(accountInfo).latestConfirmedRound.stdDeviation,
    );

    return {
      price,
      lastUpdatedSlot,
      uiDeviation: stdDeviation.toNumber(),
      provider: OracleProvider.Switchboard,
    };
    // if oracle is badly configured or didn't publish price at least once
    // decodeLatestAggregatorValue can throw (0 switchboard rounds).
  } catch (e) {
    console.log(`Unable to parse Switchboard Oracle V2: ${oracle}`, e);
    return {
      price: 0,
      lastUpdatedSlot: 0,
      uiDeviation: 0,
      provider: OracleProvider.Switchboard,
    };
  }
}

export function getStandardDeviation(array: number[]): number {
  const n = array.length;
  const mean = array.reduce((a, b) => a + b) / n;
  return Math.sqrt(
    array.map((x) => Math.pow(x - mean, 2)).reduce((a, b) => a + b) / n,
  );
}

export function parseSwitchboardOnDemandOracle(
  program: any,
  accountInfo: AccountInfo<Buffer>,
  oracle: PublicKey,
): {
  price: number;
  lastUpdatedSlot: number;
  uiDeviation: number;
  provider: OracleProvider;
} {
  try {
    const decodedPullFeed = program.coder.accounts.decode(
      'pullFeedAccountData',
      accountInfo.data,
    );

    // useful for development
    // console.log(decodedPullFeed);
    // console.log(decodedPullFeed.submissions);

    // Use custom code instead of toFeedValue from sb on demand sdk
    // Custom code which has uses min sample size
    // const feedValue = toFeedValue(decodedPullFeed.submissions, new BN(0));

    // filter empty and sort by slot to get latest submissions in slice later
    const submissions = decodedPullFeed.submissions
      .filter((s) => s.slot != 0)
      .sort((x, y) => y.slot - x.slot);

    // submissions.map((s) => console.log(`- ${s.slot}`));

    let values = submissions.slice(0, decodedPullFeed.minSampleSize);
    if (values.length === 0) {
      return {
        price: 0,
        lastUpdatedSlot: 0,
        uiDeviation: 0,
        provider: OracleProvider.SwitchboardOnDemand,
      };
    }
    values = values.sort((x, y) => (x.value.lt(y.value) ? -1 : 1));
    const feedValue = values[Math.floor(values.length / 2)];
    const price = new Big(feedValue.value.toString()).div(1e18);
    const lastUpdatedSlot = feedValue.slot.toNumber();
    const stdDeviation = getStandardDeviation(
      values.map((val: any) =>
        new Big(val.value.toString()).div(1e18).toNumber(),
      ),
    );
    return {
      price,
      lastUpdatedSlot,
      uiDeviation: stdDeviation,
      provider: OracleProvider.SwitchboardOnDemand,
    };

    // old block, we prefer above block since we want raw data, .result is often empty
    // const price = new Big(decodedPullFeed.result.value.toString()).div(1e18);
    // const lastUpdatedSlot = decodedPullFeed.result.slot.toNumber();
    // const stdDeviation = decodedPullFeed.result.stdDev.toNumber();
    // return { price, lastUpdatedSlot, uiDeviation: stdDeviation };
  } catch (e) {
    console.log(
      `Unable to parse Switchboard On-Demand Oracle V2: ${oracle}`,
      e,
    );
    return {
      price: 0,
      lastUpdatedSlot: 0,
      uiDeviation: 0,
      provider: OracleProvider.SwitchboardOnDemand,
    };
  }
}

export async function parseSwitchboardOracle(
  oracle: PublicKey,
  accountInfo: AccountInfo<Buffer>,
  connection: Connection,
): Promise<{
  price: number;
  lastUpdatedSlot: number;
  uiDeviation: number;
  provider: OracleProvider;
}> {
  if (accountInfo.owner.equals(ON_DEMAND_MAINNET_PID)) {
    if (!sbOnDemandProgram) {
      const options = AnchorProvider.defaultOptions();
      const provider = new AnchorProvider(connection, null as any, options);
      const idl = await Anchor30Program.fetchIdl(
        ON_DEMAND_MAINNET_PID,
        provider,
      );
      sbOnDemandProgram = new Anchor30Program(idl!, provider);
    }
    return parseSwitchboardOnDemandOracle(
      sbOnDemandProgram,
      accountInfo,
      oracle,
    );
  }

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
    accountInfo.owner.equals(SwitchboardProgram.mainnetPid) ||
    accountInfo.owner.equals(ON_DEMAND_MAINNET_PID)
  ) {
    return true;
  }
  return false;
}

export function isPythOracle(accountInfo: AccountInfo<Buffer>): boolean {
  if (accountInfo.owner.equals(DEFAULT_RECEIVER_PROGRAM_ID)) {
    return true;
  }
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

export function parsePythOracle(
  accountInfo: AccountInfo<Buffer>,
  connection: Connection,
): {
  price: number;
  lastUpdatedSlot: number;
  uiDeviation: number;
  provider: OracleProvider;
} {
  if (accountInfo.owner.equals(DEFAULT_RECEIVER_PROGRAM_ID)) {
    if (!pythSolanaReceiverProgram) {
      const options = AnchorProvider.defaultOptions();
      const provider = new AnchorProvider(connection, null as any, options);
      pythSolanaReceiverProgram = new Program<PythSolanaReceiverProgram>(
        IDL as PythSolanaReceiverProgram,
        DEFAULT_RECEIVER_PROGRAM_ID,
        provider,
      );
    }

    const decoded = pythSolanaReceiverProgram.coder.accounts.decode(
      'priceUpdateV2',
      accountInfo.data,
    );

    return {
      price: toUiDecimals(
        decoded.priceMessage.price.toNumber(),
        -decoded.priceMessage.exponent,
      ),
      publishedTime: decoded.priceMessage.publishTime.toNumber(),
      lastUpdatedSlot: decoded.postedSlot.toNumber(),
      uiDeviation: toUiDecimals(
        decoded.priceMessage.conf.toNumber(),
        -decoded.priceMessage.exponent,
      ),
      provider: OracleProvider.PythV2,
    } as any;
  }

  if (accountInfo.data.readUInt32LE(0) === PythMagic) {
    const priceData = parsePriceData(accountInfo.data);
    return {
      price: priceData.previousPrice,
      lastUpdatedSlot: parseInt(priceData.lastSlot.toString()),
      uiDeviation: priceData.previousConfidence,
      provider: OracleProvider.Pyth,
    };
  }

  throw new Error('Unknown Pyth oracle!');
}

export function isOracleStaleOrUnconfident(
  nowSlot: number,
  maxStalenessSlots: number,
  oracleLastUpdatedSlot: number | undefined,
  deviation: I80F48 | undefined,
  confFilter: I80F48,
  price: I80F48,
  debug = false,
  debugPrefix: string | undefined = undefined,
): boolean {
  if (
    maxStalenessSlots >= 0 &&
    oracleLastUpdatedSlot &&
    nowSlot > oracleLastUpdatedSlot + maxStalenessSlots
  ) {
    return true;
  }
  if (debug && oracleLastUpdatedSlot) {
    console.log(
      `- ${debugPrefix?.padStart(30)}: not stale for ${oracleLastUpdatedSlot + maxStalenessSlots - nowSlot} future slots`,
    );
  }

  if (deviation && deviation.gt(confFilter.mul(price))) {
    return true;
  }
  if (debug && deviation) {
    console.log(
      `- ${debugPrefix?.padStart(30)}: deviation within confidence tolerance ${deviation.div(price).mul(I80F48.fromNumber(100)).toFixed(3)}%`,
    );
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
