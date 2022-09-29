import { Magic as PythMagic } from '@pythnetwork/client';
import { AccountInfo, PublicKey } from '@solana/web3.js';
import {
  loadSwitchboardProgram,
  SBV2_DEVNET_PID,
  SBV2_MAINNET_PID,
  SwitchboardDecimal,
} from '@switchboard-xyz/switchboard-v2';
import BN from 'bn.js';
import { I80F48, I80F48Dto } from './I80F48';

const SBV1_DEVNET_PID = new PublicKey(
  '7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU',
);
const SBV1_MAINNET_PID = new PublicKey(
  'DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM',
);
let sbv2DevnetProgram;
let sbv2MainnetProgram;

export class StubOracle {
  public price: I80F48;
  public lastUpdated: number;

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
    this.lastUpdated = lastUpdated.toNumber();
  }
}

// https://gist.github.com/microwavedcola1/b741a11e6ee273a859f3ef00b35ac1f0
export function parseSwitcboardOracleV1(
  accountInfo: AccountInfo<Buffer>,
): number {
  return accountInfo.data.readDoubleLE(1 + 32 + 4 + 4);
}

export function parseSwitcboardOracleV2(
  program,
  accountInfo: AccountInfo<Buffer>,
): number {
  const aggregatorAccountData = (program as any)._coder.accounts.decode(
    (program.account.aggregatorAccountData as any)._idlAccount.name,
    accountInfo.data,
  );
  const sbDecimal = SwitchboardDecimal.from(
    aggregatorAccountData.latestConfirmedRound.result,
  );
  return sbDecimal.toBig().toNumber();
}

/**
 *
 * @param accountInfo
 * @returns ui price
 */
export async function parseSwitchboardOracle(
  accountInfo: AccountInfo<Buffer>,
): Promise<number> {
  if (accountInfo.owner.equals(SBV2_DEVNET_PID)) {
    if (!sbv2DevnetProgram) {
      sbv2DevnetProgram = await loadSwitchboardProgram('devnet');
    }
    return parseSwitcboardOracleV2(sbv2DevnetProgram, accountInfo);
  }

  if (accountInfo.owner.equals(SBV2_MAINNET_PID)) {
    if (!sbv2MainnetProgram) {
      sbv2MainnetProgram = await loadSwitchboardProgram('mainnet-beta');
    }
    return parseSwitcboardOracleV2(sbv2MainnetProgram, accountInfo);
  }

  if (
    accountInfo.owner.equals(SBV1_DEVNET_PID) ||
    accountInfo.owner.equals(SBV1_MAINNET_PID)
  ) {
    return parseSwitcboardOracleV1(accountInfo);
  }

  throw new Error(`Should not be reached!`);
}

export function isSwitchboardOracle(accountInfo: AccountInfo<Buffer>): boolean {
  if (
    accountInfo.owner.equals(SBV1_DEVNET_PID) ||
    accountInfo.owner.equals(SBV1_MAINNET_PID) ||
    accountInfo.owner.equals(SBV2_DEVNET_PID) ||
    accountInfo.owner.equals(SBV2_MAINNET_PID)
  ) {
    return true;
  }
  return false;
}

export function isPythOracle(accountInfo: AccountInfo<Buffer>): boolean {
  return accountInfo.data.readUInt32LE(0) === PythMagic;
}
