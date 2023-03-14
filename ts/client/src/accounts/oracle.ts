import { Magic as PythMagic } from '@pythnetwork/client';
import { AccountInfo, Connection, PublicKey } from '@solana/web3.js';
import SwitchboardProgram from '@switchboard-xyz/sbv2-lite';
import BN from 'bn.js';
import { I80F48, I80F48Dto } from '../numbers/I80F48';

const SBV1_DEVNET_PID = new PublicKey(
  '7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU',
);
const SBV1_MAINNET_PID = new PublicKey(
  'DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM',
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
  public lastUpdated: BN;

  static from(
    publicKey: PublicKey,
    obj: {
      group: PublicKey;
      mint: PublicKey;
      price: I80F48Dto;
      lastUpdated: BN;
    },
  ): StubOracle {
    return new StubOracle(
      publicKey,
      obj.group,
      obj.mint,
      obj.price,
      obj.lastUpdated,
    );
  }

  constructor(
    public publicKey: PublicKey,
    public group: PublicKey,
    public mint: PublicKey,
    price: I80F48Dto,
    lastUpdated: BN,
  ) {
    this.price = I80F48.from(price);
    this.lastUpdated = lastUpdated;
  }
}

// https://gist.github.com/microwavedcola1/b741a11e6ee273a859f3ef00b35ac1f0
export function parseSwitchboardOracleV1(accountInfo: AccountInfo<Buffer>): {
  price: number;
  lastUpdatedSlot: number;
} {
  const price = accountInfo.data.readDoubleLE(1 + 32 + 4 + 4);
  const lastUpdatedSlot = parseInt(
    accountInfo.data.readBigUInt64LE(1 + 32 + 4 + 4 + 8).toString(),
  );
  return { price, lastUpdatedSlot };
}

export function parseSwitchboardOracleV2(
  program: SwitchboardProgram,
  accountInfo: AccountInfo<Buffer>,
): { price: number; lastUpdatedSlot: number } {
  const price = program.decodeLatestAggregatorValue(accountInfo)!.toNumber();
  const lastUpdatedSlot = program
    .decodeAggregator(accountInfo)
    .latestConfirmedRound!.roundOpenSlot!.toNumber();

  if (!price || !lastUpdatedSlot)
    throw new Error('Unable to parse Switchboard Oracle V2');
  return { price, lastUpdatedSlot };
}

/**
 *
 * @param accountInfo
 * @returns ui price
 */
export async function parseSwitchboardOracle(
  accountInfo: AccountInfo<Buffer>,
  connection: Connection,
): Promise<{ price: number; lastUpdatedSlot: number }> {
  if (accountInfo.owner.equals(SwitchboardProgram.devnetPid)) {
    if (!sbv2DevnetProgram) {
      sbv2DevnetProgram = await SwitchboardProgram.loadDevnet(connection);
    }
    return parseSwitchboardOracleV2(sbv2DevnetProgram, accountInfo);
  }

  if (accountInfo.owner.equals(SwitchboardProgram.mainnetPid)) {
    if (!sbv2MainnetProgram) {
      sbv2MainnetProgram = await SwitchboardProgram.loadMainnet(connection);
    }
    return parseSwitchboardOracleV2(sbv2MainnetProgram, accountInfo);
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
